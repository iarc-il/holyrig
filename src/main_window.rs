use crate::rig::Rig;
use crate::rig::{self, RigModel};

use relm4::Controller;
use relm4::gtk::{Notebook, glib};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, RelmWidgetExt,
    SimpleComponent, component,
    gtk::{self, prelude::*},
};

#[derive(Debug)]
pub enum MainWindowMessage {
    SwitchTab(u32),
    NewRig,
    Quit,
}

pub struct MainWindowModel {
    tab_number: u32,
    rigs: Vec<Controller<RigModel>>,
    notebook: Notebook,
}

#[component(pub)]
impl SimpleComponent for MainWindowModel {
    type Input = MainWindowMessage;
    type Output = ();
    type Init = ();

    view! {
        gtk::Window {
            set_title: Some("Holyrig"),
            set_default_width: 300,
            set_default_height: 50,
            set_resizable: false,
            connect_close_request[sender] => move |_| {
                sender.input(MainWindowMessage::Quit);
                glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 2,
                set_margin_all: 2,

                #[local_ref]
                notebook -> gtk::Notebook {
                    connect_switch_page[sender] => move |_, _, page_num| {
                        sender.input(MainWindowMessage::SwitchTab(page_num));
                    }
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 4,
                    set_margin_all: 4,
                    set_align: gtk::Align::End,

                    gtk::Button::with_label("Cancel") {},
                    gtk::Button::with_label("OK") {
                        connect_clicked => MainWindowMessage::NewRig,
                    },

                }
            }
        }
    }

    fn init(
        _counter: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let notebook = gtk::Notebook::new();
        let model = MainWindowModel {
            tab_number: 0,
            rigs: vec![],
            notebook: notebook.clone(),
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            MainWindowMessage::SwitchTab(tab_number) => {
                self.tab_number = tab_number;
            }
            MainWindowMessage::NewRig => {
                let rig_controller = rig::RigModel::builder().launch(Rig::default()).detach();
                let rig_widget = rig_controller.widget();
                let label = format!("RIG {}", self.rigs.len() + 1);
                self.notebook
                    .append_page(rig_widget, Some(&gtk::Label::new(Some(label.as_str()))));
                self.rigs.push(rig_controller)
            }
            MainWindowMessage::Quit => {
                // Quit gracefully and drop all controllers
                relm4::main_application().quit();
            }
        }
    }
}
