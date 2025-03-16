use relm4::RelmApp;

mod main_window;
mod rig;

fn main() {
    let app = RelmApp::new("org.iarc.holyrig");
    app.run::<main_window::MainWindowModel>(());
}
