use crate::quadra::Message;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{Clipboard, Shell, mouse};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme};

pub struct FlatStack<'a> {
    layers: Vec<Element<'a, Message>>,
}

pub fn flat_stack<'a>(layers: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    Element::new(FlatStack { layers })
}

impl Widget<Message, Theme, Renderer> for FlatStack<'_> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn children(&self) -> Vec<Tree> {
        self.layers.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.layers);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let layers = self
            .layers
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(layer, tree)| {
                layer
                    .as_widget_mut()
                    .layout(tree, renderer, &layout::Limits::new(Size::ZERO, size))
            })
            .collect();
        layout::Node::with_children(size, layers)
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
        for ((layer, tree), layout) in self
            .layers
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            layer.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
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
        for ((layer, tree), layout) in self
            .layers
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            layer
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        }
    }
}
