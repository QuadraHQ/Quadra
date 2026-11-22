use crate::quadra::Message;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{Clipboard, Shell, mouse};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme};

const ON_GPU: usize = 0;
const ON_SOFTWARE: usize = 1;

pub struct GpuOr<'a> {
    choices: [Element<'a, Message>; 2],
}

pub fn gpu_or<'a>(
    on_gpu: impl Into<Element<'a, Message>>,
    on_software: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    Element::new(GpuOr {
        choices: [on_gpu.into(), on_software.into()],
    })
}

fn chosen_for(renderer: &Renderer) -> usize {
    match renderer {
        Renderer::Primary(_) => ON_GPU,
        Renderer::Secondary(_) => ON_SOFTWARE,
    }
}

impl Widget<Message, Theme, Renderer> for GpuOr<'_> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn children(&self) -> Vec<Tree> {
        self.choices.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.choices);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let chosen = chosen_for(renderer);
        let child = self.choices[chosen].as_widget_mut().layout(
            &mut tree.children[chosen],
            renderer,
            limits,
        );
        let size = limits.resolve(Length::Fill, Length::Fill, child.size());
        layout::Node::with_children(size, vec![child])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let chosen = chosen_for(renderer);
        let Some(child_layout) = layout.children().next() else {
            return;
        };
        self.choices[chosen].as_widget_mut().update(
            &mut tree.children[chosen],
            event,
            child_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let chosen = chosen_for(renderer);
        let Some(child_layout) = layout.children().next() else {
            return;
        };
        self.choices[chosen].as_widget().draw(
            &tree.children[chosen],
            renderer,
            theme,
            style,
            child_layout,
            cursor,
            viewport,
        );
    }
}
