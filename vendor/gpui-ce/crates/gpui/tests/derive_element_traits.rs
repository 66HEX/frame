use gpui::{
    AnyElement, Empty, Interactivity, IntoElement, ParentElement, StatefulInteractiveElement,
    StyleRefinement, Styled,
};

#[derive(gpui::Styled, gpui::ParentElement)]
struct Card {
    #[style]
    style: StyleRefinement,
    #[children]
    children: Vec<AnyElement>,
}

#[derive(gpui::InteractiveElement, gpui::StatefulInteractiveElement)]
struct Control {
    #[interactivity]
    interactivity: Interactivity,
}

fn requires_stateful<Type: StatefulInteractiveElement>(element: &mut Type) -> &mut Interactivity {
    element.interactivity()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_element_traits() {
        let mut card = Card {
            style: StyleRefinement::default(),
            children: Vec::new(),
        };

        let style: &mut StyleRefinement = card.style();
        style.opacity = Some(0.5);
        card.extend([Empty.into_any_element()]);
        assert_eq!(card.children.len(), 1);

        let mut control = Control {
            interactivity: Interactivity::default(),
        };

        let interactivity: &mut Interactivity = requires_stateful(&mut control);
        assert!(std::ptr::eq(interactivity, &control.interactivity));
    }
}
