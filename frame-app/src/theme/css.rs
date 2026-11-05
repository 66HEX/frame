//! Build-time import of the same :root/.dark custom properties used by shadcn.
use csscolorparser::Color;
use serde_json::{Value, json};
use std::collections::BTreeMap;

type Variables = BTreeMap<String, String>;
const REQUIRED: &[&str] = &[
    "background",
    "foreground",
    "card",
    "card-foreground",
    "popover",
    "popover-foreground",
    "primary",
    "primary-foreground",
    "secondary",
    "secondary-foreground",
    "muted",
    "muted-foreground",
    "accent",
    "accent-foreground",
    "destructive",
    "border",
    "input",
    "ring",
];

pub struct ImportedTheme {
    pub colors: Value,
    pub radius: f32,
}

pub fn import(css: &str) -> Result<ImportedTheme, String> {
    let css = strip_comments(css)?;
    let mut light = Variables::new();
    let mut dark = Variables::new();
    collect_blocks(&css, &mut light, &mut dark)?;
    if light.is_empty() {
        return Err("missing :root { ... } theme block".into());
    }
    let radius = radius(&resolve("radius", &light, &mut Vec::new())?)?;
    let mut dark_variables = light.clone();
    dark_variables.extend(dark);
    let dark_radius = radius_value(&dark_variables)?;
    if (radius - dark_radius).abs() > f32::EPSILON {
        return Err("--radius must be the same in :root and .dark".into());
    }
    Ok(ImportedTheme {
        colors: json!({"light": colors(&light)?, "dark": colors(&dark_variables)?}),
        radius,
    })
}

fn radius_value(variables: &Variables) -> Result<f32, String> {
    radius(&resolve("radius", variables, &mut Vec::new())?)
}

fn radius(value: &str) -> Result<f32, String> {
    let (number, factor) = if let Some(value) = value.strip_suffix("rem") {
        (value, 16.0)
    } else if let Some(value) = value.strip_suffix("px") {
        (value, 1.0)
    } else if value == "0" {
        (value, 1.0)
    } else {
        return Err(format!("--radius: expected rem, px or 0, got {value:?}"));
    };
    let value = number.trim().parse::<f32>().map_err(|e| e.to_string())? * factor;
    if !value.is_finite() || value < 0.0 {
        return Err("--radius must be a finite nonnegative length".into());
    }
    Ok(value)
}

fn strip_comments(css: &str) -> Result<String, String> {
    let mut rest = css;
    let mut output = String::new();
    while let Some((before, after)) = rest.split_once("/*") {
        output.push_str(before);
        output.push(' ');
        rest = after.split_once("*/").ok_or("unclosed CSS comment")?.1;
    }
    output.push_str(rest);
    Ok(output)
}

fn collect_blocks(css: &str, light: &mut Variables, dark: &mut Variables) -> Result<(), String> {
    let mut rest = css;
    while let Some((selector, block)) = rest.split_once('{') {
        let mut depth = 1;
        let mut end = None;
        for (index, ch) in block.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                end = Some(index);
                break;
            }
        }
        let end = end.ok_or("unclosed CSS block")?;
        let body = &block[..end];
        // Imports before a selector and @layer base wrappers are accepted.
        let selector = selector.rsplit(';').next().unwrap_or(selector).trim();
        if selector.starts_with('@') {
            collect_blocks(body, light, dark)?;
        } else {
            for selector in selector.split(',').map(str::trim) {
                let target = match selector {
                    ":root" => &mut *light,
                    ".dark" | ":root.dark" | "html.dark" => &mut *dark,
                    _ => continue,
                };
                for declaration in body.split(';') {
                    if let Some((name, value)) = declaration.split_once(':')
                        && let Some(name) = name.trim().strip_prefix("--")
                    {
                        target.insert(
                            name.into(),
                            value.trim().trim_end_matches("!important").trim().into(),
                        );
                    }
                }
            }
        }
        rest = &block[end + 1..];
    }
    Ok(())
}

fn resolve(name: &str, variables: &Variables, stack: &mut Vec<String>) -> Result<String, String> {
    if stack.iter().any(|item| item == name) || stack.len() >= 32 {
        return Err(format!("cyclic CSS variable: --{name}"));
    }
    let mut value = variables
        .get(name)
        .ok_or_else(|| format!("missing --{name}"))?
        .clone();
    stack.push(name.into());
    while let Some(start) = value.find("var(") {
        let mut depth = 1;
        let end = value[start + 4..]
            .char_indices()
            .find_map(|(index, ch)| {
                if ch == '(' {
                    depth += 1;
                }
                if ch == ')' {
                    depth -= 1;
                }
                (depth == 0).then_some(start + 4 + index)
            })
            .ok_or("unclosed var()")?;
        let (reference, fallback) = value[start + 4..end]
            .split_once(',')
            .map_or((&value[start + 4..end], None), |(a, b)| (a, Some(b.trim())));
        let reference = reference
            .trim()
            .strip_prefix("--")
            .ok_or("var() must reference --name")?;
        let replacement = if variables.contains_key(reference) {
            resolve(reference, variables, stack)?
        } else {
            fallback
                .ok_or_else(|| format!("missing --{reference}, referenced by --{name}"))?
                .into()
        };
        value.replace_range(start..=end, &replacement);
    }
    stack.pop();
    Ok(value)
}

fn parse_color(value: &str) -> Result<Color, String> {
    csscolorparser::parse(value)
        .or_else(|_| csscolorparser::parse(&format!("hsl({value})")))
        .map_err(|error| format!("invalid CSS color {value:?}: {error}"))
}

fn colors(variables: &Variables) -> Result<Value, String> {
    let mut values = BTreeMap::new();
    for &name in REQUIRED {
        let value = resolve(name, variables, &mut Vec::new())?;
        parse_color(&value).map_err(|e| format!("--{name}: {e}"))?;
    }
    for (name, css) in variables {
        if name == "radius"
            || name.starts_with("font-")
            || name.starts_with("shadow")
            || name.starts_with("spacing")
            || name.starts_with("tracking")
        {
            continue;
        }
        let value = resolve(name, variables, &mut Vec::new())?;
        if let Ok(color) = parse_color(&value) {
            values.insert(name.clone(), json!({"css": css, "srgb": rgba(color)}));
        } else if REQUIRED.contains(&name.as_str())
            || name.starts_with("chart-")
            || name.starts_with("sidebar")
        {
            return Err(format!("--{name}: unsupported color {value:?}"));
        }
    }
    let secondary = parse_color(&resolve("secondary", variables, &mut Vec::new())?)?;
    let foreground = parse_color(&resolve("foreground", variables, &mut Vec::new())?)?;
    values.insert(
        "secondary-hover".into(),
        json!({
            "css": "color-mix(in oklch,var(--secondary),var(--foreground) 5%)",
            "srgb": rgba(mix_oklch(secondary, foreground, 0.05)),
        }),
    );
    Ok(json!(values))
}

fn rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a].map(|value| value.clamp(0.0, 1.0))
}

fn mix_oklch(a: Color, b: Color, t: f32) -> Color {
    let [l1, c1, mut h1, a1] = a.to_oklcha();
    let [l2, c2, mut h2, a2] = b.to_oklcha();
    if c1 < 0.00001 {
        h1 = h2;
    }
    if c2 < 0.00001 {
        h2 = h1;
    }
    let delta =
        (h2 - h1 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let alpha = a1.mul_add(1.0 - t, a2 * t);
    if alpha == 0.0 {
        return Color::new(0.0, 0.0, 0.0, 0.0);
    }
    Color::from_oklcha(
        ((l1 * a1).mul_add(1.0 - t, l2 * a2 * t)) / alpha,
        ((c1 * a1).mul_add(1.0 - t, c2 * a2 * t)) / alpha,
        delta.mul_add(t, h1),
        alpha,
    )
}

#[cfg(test)]
#[path = "css/tests.rs"]
mod tests;
