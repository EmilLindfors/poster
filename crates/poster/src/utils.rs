use std::cmp::min;
use std::collections::BTreeMap;
use std::str::FromStr;

use eframe::emath::{Align, Pos2};
use eframe::epaint::text::LayoutJob;
use egui::ahash::HashSet;
use egui::text::TextWrapping;
use egui::{
    Area, FontSelection, Frame, Id, InnerResponse, Key, Layout, Order, Response, RichText, Style,
    TextBuffer, Ui, WidgetText,
};
use regex::Regex;

use crate::data::{EnvironmentItemValue, EnvironmentValueType, HttpRecord};
use crate::env_func::{get_env_result, EnvFunction};
use crate::panels::HORIZONTAL_GAP;

pub fn build_rest_ui_header(hr: HttpRecord, max_char: Option<usize>, ui: &Ui) -> LayoutJob {
    let mut lb = LayoutJob {
        text: Default::default(),
        sections: Default::default(),
        wrap: TextWrapping {
            max_width: 50.0,
            max_rows: 1,
            break_anywhere: true,
            overflow_character: Some('…'),
        },
        first_row_min_height: 0.0,
        break_on_newline: false,
        halign: Align::LEFT,
        justify: false,
    };
    let style = Style::default();
    let base_url = hr
        .request
        .base_url
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .to_string();
    if base_url != "" {
        RichText::new(hr.request.method.to_string() + " ")
            .color(ui.visuals().warn_fg_color)
            .strong()
            .append_to(&mut lb, &style, FontSelection::Default, Align::Center);
        let mut new_name = "".to_string();
        if hr.name != "" {
            new_name = hr.name.to_string();
        } else {
            new_name = base_url;
        }
        match max_char {
            None => {}
            Some(size) => {
                if new_name.len() > size {
                    let len = min(new_name.len() - 1, size);
                    new_name = new_name.as_str()[0..len].to_string() + "...";
                }
            }
        }
        RichText::new(new_name)
            .color(ui.visuals().text_color())
            .append_to(&mut lb, &style, FontSelection::Default, Align::Center);
    } else {
        RichText::new("Untitled Request")
            .strong()
            .color(ui.visuals().text_color())
            .append_to(&mut lb, &style, FontSelection::Default, Align::Center);
    }
    lb
}

pub fn build_with_count_ui_header(name: String, count: usize, ui: &Ui) -> LayoutJob {
    let mut lb = LayoutJob::default();
    let mut color = ui.visuals().warn_fg_color;
    let style = Style::default();
    RichText::new(name + " ")
        .color(ui.visuals().text_color())
        .strong()
        .append_to(&mut lb, &style, FontSelection::Default, Align::Center);
    if count == usize::MAX {
        RichText::new("●").color(color.clone()).strong().append_to(
            &mut lb,
            &style,
            FontSelection::Default,
            Align::Center,
        );
    } else if count > 0 {
        RichText::new("(".to_string() + count.to_string().as_str() + ")")
            .color(color.clone())
            .strong()
            .append_to(&mut lb, &style, FontSelection::Default, Align::Center);
    }
    lb
}

pub fn left_right_panel(
    ui: &mut Ui,
    id: String,
    left: impl FnOnce(&mut Ui),
    right: impl FnOnce(&mut Ui),
) -> InnerResponse<()> {
    let left_id = id.clone() + "_left";
    let right_id = id.clone() + "_right";
    ui.horizontal(|ui| {
        egui::SidePanel::right(right_id)
            .resizable(true)
            .show_separator_line(false)
            .show_inside(ui, |ui| {
                right(ui);
            });
        egui::SidePanel::left(left_id)
            .resizable(true)
            .min_width(ui.available_width() - HORIZONTAL_GAP * 2.0)
            .show_inside(ui, |ui| {
                left(ui);
            });
    })
}

pub fn popup_widget<R>(
    ui: &Ui,
    popup_id: Id,
    widget_response: &Response,
    suggested_position: Pos2,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    if ui.memory(|mem| mem.is_popup_open(popup_id)) {
        let inner = Area::new(popup_id)
            .order(Order::Foreground)
            .constrain(true)
            .fixed_pos(suggested_position)
            .show(ui.ctx(), |ui| {
                let frame = Frame::popup(ui.style());
                frame
                    .show(ui, |ui| {
                        ui.with_layout(Layout::left_to_right(Align::LEFT), |ui| add_contents(ui))
                            .inner
                    })
                    .inner
            })
            .inner;

        if ui.input(|i| i.key_pressed(Key::Escape)) || widget_response.clicked_elsewhere() {
            ui.memory_mut(|mem| mem.close_popup());
        }
        Some(inner)
    } else {
        None
    }
}

pub fn replace_variable(content: String, envs: BTreeMap<String, EnvironmentItemValue>) -> String {
    let re = Regex::new(r"\{\{.*?}}").unwrap();
    let mut result = content.clone();
    loop {
        let temp = result.clone();
        let find = re.find_iter(temp.as_str()).next();
        match find {
            None => break,
            Some(find_match) => {
                let key = find
                    .unwrap()
                    .as_str()
                    .trim_start_matches("{{")
                    .trim_end_matches("}}");
                let v = envs.get(key);
                match v {
                    None => result.replace_range(find_match.range(), "{UNKNOWN}"),
                    Some(etv) => match etv.value_type {
                        EnvironmentValueType::String => {
                            result.replace_range(find_match.range(), etv.value.as_str())
                        }
                        EnvironmentValueType::Function => {
                            let env_func = EnvFunction::from_str(etv.value.as_str());
                            match env_func {
                                Ok(f) => result
                                    .replace_range(find_match.range(), get_env_result(f).as_str()),
                                Err(_) => {
                                    result.replace_range(find_match.range(), "{UNKNOWN}");
                                }
                            }
                        }
                    },
                }
            }
        }
    }
    result
}

pub fn select_label(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.with_layout(
        Layout::top_down(Align::LEFT).with_cross_justify(true),
        |ui| ui.selectable_label(false, text),
    )
    .inner
}

pub fn select_value<Value: PartialEq>(
    ui: &mut Ui,
    current_value: &mut Value,
    selected_value: Value,
    text: impl Into<WidgetText>,
) -> Response {
    ui.with_layout(
        Layout::top_down(Align::LEFT).with_cross_justify(true),
        |ui| ui.selectable_value(current_value, selected_value, text),
    )
    .inner
}

pub fn text_edit_singleline_justify<S: TextBuffer>(ui: &mut Ui, text: &mut S) -> Response {
    ui.with_layout(
        Layout::top_down(Align::LEFT).with_cross_justify(true),
        |ui| ui.text_edit_singleline(text),
    )
    .inner
}

pub fn text_edit_singleline_filter_justify<S: TextBuffer>(ui: &mut Ui, text: &mut S) -> Response {
    text.replace(
        text.as_str()
            .replace("/", "_")
            .as_str()
            .replace(" ", "_")
            .as_str(),
    );
    let filtered_string: String = text
        .as_str()
        .chars()
        .filter(|&c| c.is_ascii_alphabetic() || c.is_alphabetic() || c.is_numeric() || c == '_')
        .collect();
    text.replace(filtered_string.as_str());
    ui.with_layout(
        Layout::top_down(Align::LEFT).with_cross_justify(true),
        |ui| ui.text_edit_singleline(text),
    )
    .inner
}

pub fn text_edit_singleline_filter<S: TextBuffer>(ui: &mut Ui, text: &mut S) -> Response {
    text.replace(
        text.as_str()
            .replace("/", "_")
            .as_str()
            .replace(" ", "_")
            .as_str(),
    );
    let filtered_string: String = text
        .as_str()
        .chars()
        .filter(|&c| c.is_ascii_alphabetic() || c.is_alphabetic() || c.is_numeric() || c == '_')
        .collect();
    text.replace(filtered_string.as_str());
    ui.text_edit_singleline(text)
}

pub fn text_edit_multiline_justify<S: TextBuffer>(ui: &mut Ui, text: &mut S) -> Response {
    ui.with_layout(
        Layout::top_down(Align::LEFT).with_cross_justify(true),
        |ui| ui.text_edit_multiline(text),
    )
    .inner
}

pub fn build_copy_name(mut name: String, names: HashSet<String>) -> String {
    name = name.splitn(2, "Copy").next().unwrap().trim().to_string();
    let mut index = 2;
    let mut new_name = name.clone();
    while (names.contains(new_name.as_str())) {
        new_name = format!("{} Copy {}", name.clone(), index);
        index += 1;
    }
    return new_name;
}

pub fn selectable_check<Value: PartialEq>(
    ui: &mut Ui,
    current_value: &mut Value,
    selected_value: Value,
    text: impl Into<WidgetText>,
) -> Response {
    let mut response = ui.checkbox(&mut (*current_value == selected_value), text);
    if response.clicked() && *current_value != selected_value {
        *current_value = selected_value;
        response.mark_changed();
    }
    response
}
