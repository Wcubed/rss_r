use egui::output::OpenUrl;
use egui::{Link, Response, Ui, Widget, WidgetText};

/// The standard egui hyperlink opens the link in the current page,
/// and opens in a new tab if `ctrl`, or middle mouse is pressed.
/// This version always opens in a new tab.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct NewTabHyperlink {
    url: String,
    text: WidgetText,
}

impl NewTabHyperlink {
    pub fn from_label_and_url(text: impl Into<WidgetText>, url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            text: text.into(),
        }
    }
}

impl Widget for NewTabHyperlink {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { url, text } = self;

        let response = ui.add(Link::new(text));
        if response.clicked() {
            ui.ctx().output_mut(|output| {
                output.open_url = Some(OpenUrl {
                    url: url.clone(),
                    new_tab: true,
                })
            });
        }
        response.on_hover_text(url)
    }
}
