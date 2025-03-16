use crate::rig::{self, Rig};

use relm4::{
    component,
    gtk::{self, prelude::*},
    Component, ComponentController, ComponentParts, ComponentSender, RelmWidgetExt,
    SimpleComponent,
};

#[derive(Debug)]
pub enum MainWindowMessage {
    SwitchTab(u32),
}

pub struct MainWindowModel {
    tab_number: u32,
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
                    gtk::Button::with_label("OK") {},

                }
            }
        }
    }

    fn init(
        _counter: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = MainWindowModel { tab_number: 0 };

        let notebook = gtk::Notebook::new();

        let rig = rig::RigModel::builder().launch(Rig::new()).detach();
        let rig_widget = rig.widget();
        notebook.append_page(rig_widget, Some(&gtk::Label::new(Some("RIG 1"))));

        let rig = rig::RigModel::builder().launch(Rig::new()).detach();
        let rig_widget = rig.widget();
        notebook.append_page(rig_widget, Some(&gtk::Label::new(Some("RIG 2"))));

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            MainWindowMessage::SwitchTab(tab_number) => {
                self.tab_number = tab_number;
            }
        }
    }
}
