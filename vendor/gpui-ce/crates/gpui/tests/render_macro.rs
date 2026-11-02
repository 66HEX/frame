use gpui_macros::Render;

#[derive(Render)]
struct Element;

#[test]
fn derives_render() {
    let _ = Element;
}
