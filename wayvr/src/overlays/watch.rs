use crate::{
    gui::{
        panel::{
            GuiPanel, NewGuiPanelParams, apply_custom_command, device_list::DeviceList,
            overlay_list::OverlayList, set_list::SetList,
        },
        timer::GuiTimer,
    },
    state::AppState,
    windowing::{Z_ORDER_WATCH, backend::OverlayEventData, window::OverlayWindowConfig},
};
use glam::{Affine3A, Quat, Vec3, vec3};
use std::{cell::Cell, rc::Rc, time::Duration};
use wgui::{
    assets::AssetPathRef,
    components::{button::ComponentButton, slider::ComponentSlider},
    parser::{Fetchable, ParseDocumentParams},
};
use wlx_common::{
    common::LeftRight,
    windowing::{OverlayWindowState, Positioning},
};

pub const WATCH_NAME: &str = "watch";
pub const WATCH_POS: Vec3 = vec3(-0.03, -0.01, 0.125);
pub const WATCH_ROT: Quat = Quat::from_xyzw(-0.707_106_6, 0.000_796_361_8, 0.707_106_6, 0.0);

#[derive(Default)]
struct WatchState {
    device_list: DeviceList,
    overlay_list: OverlayList,
    set_list: SetList,
    clock_12h: bool,
}

pub fn create_watch(app: &mut AppState) -> anyhow::Result<OverlayWindowConfig> {
    let state = WatchState {
        clock_12h: app.session.config.clock_12h,
        ..Default::default()
    };
    let watch_xml = "gui/watch.xml";

    let mut panel =
        GuiPanel::new_from_template(app, watch_xml, state, NewGuiPanelParams::default())?;

    sets_or_overlays(&mut panel, app);
    setup_xrizer_controls(&mut panel);

    let doc_params = ParseDocumentParams {
        globals: panel.layout.state.globals.clone(),
        path: AssetPathRef::FileOrBuiltIn(watch_xml),
        extra: panel.doc_extra.take().unwrap_or_default(),
    };

    panel.on_notify = Some(Box::new({
        let name = WATCH_NAME;
        move |panel, app, event_data| {
            let mut elems_changed = panel.state.overlay_list.on_notify(
                &mut panel.layout,
                &mut panel.parser_state,
                &event_data,
                &doc_params,
            )?;

            elems_changed |= panel.state.set_list.on_notify(
                &mut panel.layout,
                &mut panel.parser_state,
                &event_data,
                &doc_params,
            )?;

            elems_changed |= panel.state.device_list.on_notify(
                app,
                &mut panel.layout,
                &mut panel.parser_state,
                &event_data,
                &doc_params,
            )?;

            match event_data {
                OverlayEventData::EditModeChanged(edit_mode) => {
                    if let Ok(btn_edit_mode) = panel
                        .parser_state
                        .fetch_component_as::<ComponentButton>("btn_edit_mode")
                    {
                        btn_edit_mode.set_sticky_state(&mut panel.layout.common(), edit_mode);
                    }
                }
                OverlayEventData::SettingsChanged => {
                    panel.layout.mark_redraw();
                    sets_or_overlays(panel, app);

                    if app.session.config.clock_12h != panel.state.clock_12h {
                        panel.state.clock_12h = app.session.config.clock_12h;

                        let clock_root = panel.parser_state.get_widget_id("clock_root")?;
                        panel.layout.remove_children(clock_root);

                        panel.parser_state.instantiate_template(
                            &doc_params,
                            "Clock",
                            &mut panel.layout,
                            clock_root,
                            Default::default(),
                        )?;

                        elems_changed = true;
                    }
                }
                OverlayEventData::CustomCommand { element, command } => {
                    if let Err(e) = apply_custom_command(panel, app, &element, &command) {
                        log::warn!("Could not apply {command:?} on {name}/{element}: {e:?}");
                    } else {
                        elems_changed = true;
                    }
                }
                _ => {}
            }

            if elems_changed {
                panel.process_custom_elems(app);
            }

            Ok(())
        }
    }));

    panel
        .timers
        .push(GuiTimer::new(Duration::from_millis(100), 0));

    let positioning = Positioning::FollowHand {
        hand: LeftRight::Left,
        lerp: 1.0,
    };

    panel.update_layout(app)?;

    Ok(OverlayWindowConfig {
        name: WATCH_NAME.into(),
        z_order: Z_ORDER_WATCH,
        default_state: OverlayWindowState {
            grabbable: false,
            interactable: true,
            positioning,
            transform: Affine3A::from_scale_rotation_translation(
                Vec3::ONE * 0.115,
                WATCH_ROT,
                WATCH_POS,
            ),
            angle_fade: true,
            ..OverlayWindowState::default()
        },
        show_on_spawn: app.session.config.enable_watch,
        global: true,
        ..OverlayWindowConfig::from_backend(Box::new(panel))
    })
}

fn setup_xrizer_controls(panel: &mut GuiPanel<WatchState>) {
    // Toggle button: show/hide slider container (method 2, default hidden).
    match panel
        .parser_state
        .fetch_component_as::<ComponentButton>("btn_xrizer")
    {
        Ok(btn) => {
            let root_id = panel
                .parser_state
                .get_widget_id("xrizer_root")
                .unwrap_or_default();
            let visible = Rc::new(Cell::new(false));
            {
                let mut com = panel.layout.common();
                com.alterables.set_widget_visible(root_id, false);
                btn.set_sticky_state(&mut com, false);
            }
            let btn_clone = btn.clone();
            btn.on_click(Rc::new(move |common, _| {
                let next = !visible.get();
                visible.set(next);
                common.alterables.set_widget_visible(root_id, next);
                btn_clone.set_sticky_state(common, next);
                common.alterables.mark_redraw();
                Ok(())
            }));
        }
        Err(e) => log::warn!("xrizer: btn_xrizer not found: {e:?}"),
    }

    // Scale slider: 0.01..=1.40, terminal sink is plain file write.
    match panel
        .parser_state
        .fetch_component_as::<ComponentSlider>("xrizer_scale_slider")
    {
        Ok(slider) => {
            let mut com = panel.layout.common();
            slider.set_value_primary(&mut com, crate::xrizer::read_scale());
            slider.on_value_changed(Box::new(move |_common, e| {
                crate::xrizer::write_scale(e.value);
            }));
        }
        Err(e) => log::warn!("xrizer: scale slider not found: {e:?}"),
    }

    // Brightness slider: 0.00..=1.00.
    match panel
        .parser_state
        .fetch_component_as::<ComponentSlider>("xrizer_brightness_slider")
    {
        Ok(slider) => {
            let mut com = panel.layout.common();
            slider.set_value_primary(&mut com, crate::xrizer::read_brightness());
            slider.on_value_changed(Box::new(move |_common, e| {
                crate::xrizer::write_brightness(e.value);
            }));
        }
        Err(e) => log::warn!("xrizer: brightness slider not found: {e:?}"),
    }
}

fn sets_or_overlays(panel: &mut GuiPanel<WatchState>, app: &mut AppState) {
    let visible = if app.session.config.sets_on_watch {
        [false, true]
    } else {
        [true, false]
    };

    let widget = [
        panel
            .parser_state
            .get_widget_id("panels_root")
            .unwrap_or_default(),
        panel
            .parser_state
            .get_widget_id("sets_root")
            .unwrap_or_default(),
    ];

    for i in 0..2 {
        panel
            .layout
            .alterables
            .set_widget_visible(widget[i], visible[i]);
    }
}
