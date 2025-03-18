use eframe::egui;
use egui::Ui;
use egui_dock::{DockArea, DockState, Style, TabViewer};
use rig::Rig;

mod rig;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([300.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native("Holyrig", options, Box::new(|_| Ok(Box::new(App::new()))))
}

struct AppTabViewer;

impl TabViewer for AppTabViewer {
    type Tab = Rig;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        format!("RIG {:?}", tab.rig_type).as_str().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.label(format!("THIS IS {:?}", &tab.rig_type));
    }
}

struct AppTabs {
    dock_state: DockState<Rig>,
}

impl AppTabs {
    fn new() -> Self {
        let rig = Rig::default();
        let dock_state = DockState::new(vec![rig]);
        Self { dock_state }
    }
    fn ui(&mut self, ui: &mut Ui) {
        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ui.style().as_ref()))
            .show_inside(ui, &mut AppTabViewer);
    }
}

struct App {
    tabs: AppTabs,
}

impl App {
    fn new() -> Self {
        App {
            tabs: AppTabs::new()
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.3);
        egui::CentralPanel::default().show(ctx, |ui| {
            self.tabs.ui(ui)
        });
    }
}
