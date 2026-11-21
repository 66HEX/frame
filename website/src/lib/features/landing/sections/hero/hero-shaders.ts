export const vertex = /* glsl */ `#version 300 es
in vec2 aPosition;
in vec2 aUV;
out vec2 vUV;
uniform mat3 uProjectionMatrix;
uniform mat3 uWorldTransformMatrix;
uniform mat3 uTransformMatrix;
void main() {
  vUV = aUV;
  gl_Position = vec4((uProjectionMatrix * uWorldTransformMatrix * uTransformMatrix * vec3(aPosition, 1.0)).xy, 0.0, 1.0);
}`;

const field = /* glsl */ `
uniform vec2 uSize;
uniform vec2 uView;
uniform vec2 uPointer;
uniform float uTime;
uniform float uPixelRatio;
uniform float uBloomPixelRatio;
uniform float uDark;
uniform vec3 uPrimary;
uniform vec3 uAccent;
uniform vec3 uSecondary;

float hash(vec2 p) {
  vec3 q = fract(vec3(p.xyx) * 0.1031);
  q += dot(q, q.yzx + 33.33);
  return fract((q.x + q.y) * q.z);
}

float segment(vec2 p, vec2 a, vec2 b) {
  vec2 ab = b - a;
  return length(p - a - ab * clamp(dot(p - a, ab) / dot(ab, ab), 0.0, 1.0));
}

vec2 rotate60(vec2 p) { return mat2(0.5, 0.8660254, -0.8660254, 0.5) * p; }

float glyph(vec2 p, float kind, float pixel) {
  float aa = pixel * 0.5;
  float stroke = max(0.045, pixel * 0.55);
  vec2 top = vec2(0.0, -0.34);
  vec2 left = vec2(-0.29445, 0.17);
  vec2 right = vec2(0.29445, 0.17);
  float junction = min(segment(p, vec2(0.0), top), min(segment(p, vec2(0.0), left), segment(p, vec2(0.0), right))) - stroke;
  float star = min(junction, min(segment(p, vec2(0.0), -top), min(segment(p, vec2(0.0), -left), segment(p, vec2(0.0), -right))) - stroke);
  vec2 h = abs(p);
  float hexagon = max(h.x * 0.5 + h.y * 0.8660254, h.x) - 0.295;
  float corners = 10.0;
  vec2 q = p;
  for (int i = 0; i < 3; i++) {
    corners = min(corners, min(segment(q, vec2(-0.15, -0.25), top), segment(q, top, vec2(0.15, -0.25))) - stroke);
    q = rotate60(rotate60(q));
  }
  float distance = junction;
  if (kind > 0.5) distance = star;
  if (kind > 1.5) distance = corners;
  if (kind > 2.5) distance = max(hexagon, -junction - 0.025);
  if (kind > 3.5) distance = hexagon;
  return 1.0 - smoothstep(-aa, aa, distance);
}

vec4 hexCell(vec2 p) {
  const vec2 spacing = vec2(1.0, 1.7320508);
  vec2 a = mod(p, spacing) - spacing * 0.5;
  vec2 b = mod(p - spacing * 0.5, spacing) - spacing * 0.5;
  vec2 local = dot(a, a) < dot(b, b) ? a : b;
  return vec4(local, p - local);
}

float energy(vec2 uv) {
  vec2 p = (uv - vec2(0.5, 0.1)) / vec2(1.3, 1.075);
  float radius = length(p);
  float angle = atan(p.y, p.x);
  float wave = sin(angle * 7.0 + radius * 22.0 - uTime * 0.32);
  float arc = exp(-pow((radius - 0.49 - wave * 0.018) / 0.16, 2.0));
  float ribbon = exp(-pow((radius - 0.37 + sin(angle * 3.0 + uTime * 0.17) * 0.012) / 0.038, 2.0));
  float sweep = pow(0.5 + 0.5 * sin(angle * 3.0 - uTime * 0.24 + radius * 12.0), 3.0);
  float hover = exp(-dot((uv - uPointer) * vec2(uSize.x / uSize.y, 1.0), (uv - uPointer) * vec2(uSize.x / uSize.y, 1.0)) * 60.0);
  float quiet = smoothstep(0.24, 0.35, radius);
  float edges = smoothstep(0.0, 0.035, uv.x) * smoothstep(0.0, 0.035, 1.0 - uv.x)
    * smoothstep(0.02, 0.18, uv.y) * (1.0 - smoothstep(0.87, 1.0, uv.y));
  return clamp((arc * (0.32 + 0.53 * sweep) + ribbon * 0.22 + hover * arc * 0.18) * quiet * edges, 0.0, 1.0);
}

vec4 pattern(vec2 uv, float pixelRatio) {
  float pitch = mix(9.0, 11.0, smoothstep(480.0, 1200.0, uSize.x));
  vec2 p = (uv * uSize - vec2(uSize.x * 0.5, 0.0)) / pitch;
  vec4 cell = hexCell(p);
  vec2 center = (cell.zw * pitch + vec2(uSize.x * 0.5, 0.0)) / uSize;
  float light = energy(center);
  float random = hash(cell.zw);
  float tier = floor(clamp(light * 1.35 + (random - 0.5) * 0.16, 0.0, 0.999) * 16.0);
  float kind = mod(tier, 5.0);
  if (random < 0.27) kind = floor(hash(cell.zw + 71.0) * 5.0);
  float mask = glyph(cell.xy, kind, 1.0 / (pitch * pixelRatio));
  vec3 color = mix(uPrimary, uAccent, smoothstep(0.25, 0.9, light));
  color = mix(color, uSecondary, (1.0 - light) * 0.18);
  float alpha = mask * light * mix(0.48, 0.78, uDark);
  return vec4(color * alpha, alpha);
}
`;

export const sourceFragment = /* glsl */ `#version 300 es
precision highp float;
in vec2 vUV;
out vec4 finalColor;
${field}
void main() { finalColor = pattern(vec2(vUV.x, uView.x + vUV.y * uView.y), uBloomPixelRatio); }
`;

export const blurFragment = /* glsl */ `#version 300 es
precision highp float;
in vec2 vUV;
out vec4 finalColor;
uniform sampler2D uSource;
uniform vec2 uDirection;
void main() {
  vec4 color = texture(uSource, vUV) * 0.2270270270;
  color += (texture(uSource, vUV + uDirection * 1.3846153846) + texture(uSource, vUV - uDirection * 1.3846153846)) * 0.3162162162;
  color += (texture(uSource, vUV + uDirection * 3.2307692308) + texture(uSource, vUV - uDirection * 3.2307692308)) * 0.0702702703;
  finalColor = color;
}`;

export const compositeFragment = /* glsl */ `#version 300 es
precision highp float;
in vec2 vUV;
out vec4 finalColor;
uniform sampler2D uBloom;
${field}
vec3 encodeSRGB(vec3 linear) {
  return mix(12.92 * linear, 1.055 * pow(max(linear, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), linear));
}
void main() {
  vec2 uv = vec2(vUV.x, uView.x + vUV.y * uView.y);
  vec4 sharp = pattern(uv, uPixelRatio);
  vec4 echo = pattern(uv + vec2(22.0, -13.0) / uSize, uPixelRatio) * 0.13;
  vec4 bloom = texture(uBloom, vUV) * 0.85;
  vec4 light = sharp + echo * (1.0 - sharp.a);
  light += bloom * (1.0 - light.a);
  float alpha = clamp(light.a, 0.0, 1.0);
  vec3 color = encodeSRGB(light.rgb / max(alpha, 0.00001));
  color += (hash(floor(uv * uSize * uPixelRatio)) - 0.5) / 255.0;
  finalColor = vec4(clamp(color, 0.0, 1.0) * alpha, alpha);
}`;
