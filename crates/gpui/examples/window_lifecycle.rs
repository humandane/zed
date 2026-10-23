//! Exercises retaining a hidden macOS window while an auxiliary panel is visible.

#[cfg(target_os = "macos")]
fn main() -> gpui::Result<()> {
    macos::run()
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The window_lifecycle example requires macOS.");
}

#[cfg(target_os = "macos")]
mod macos {
    use gpui::{
        Action, App, Bounds, Context, Div, FocusHandle, Global, KeyBinding, ReopenRequest,
        SharedString, Stateful, TitlebarOptions, Window, WindowBounds, WindowHandle, WindowKind,
        WindowOptions, accesskit::Role, actions, div, prelude::*, px, rgb, size,
    };
    use gpui_util::ResultExt;

    actions!(
        window_lifecycle,
        [
            Increment,
            ShowPanel,
            RestoreMain,
            CloseWindow,
            Next,
            Previous,
            Quit
        ]
    );

    #[derive(Default)]
    struct Windows {
        main: Option<WindowHandle<MainWindow>>,
        panel: Option<WindowHandle<Panel>>,
    }
    impl Global for Windows {}

    struct MainWindow {
        focus: FocusHandle,
        count: usize,
        visible_reopens: usize,
        hidden_reopens: usize,
        panel_reopens: usize,
        visible_panel_reopens: usize,
        status: SharedString,
    }

    impl MainWindow {
        fn show_panel(&mut self, _: &ShowPanel, window: &mut Window, cx: &mut Context<Self>) {
            let result = (|| -> gpui::Result<()> {
                if let Some(panel) = cx.global_mut::<Windows>().panel.take() {
                    panel.update(cx, |_, window, _| window.remove_window())?;
                }
                let count = self.count;
                let bounds = Bounds::centered(None, size(px(380.), px(300.)), cx);
                let panel = cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: None,
                        kind: WindowKind::PopUp,
                        is_resizable: false,
                        is_minimizable: false,
                        ..Default::default()
                    },
                    |window, cx| {
                        cx.new(|cx| {
                            let focus = cx.focus_handle();
                            focus.focus(window, cx);
                            Panel { focus, count }
                        })
                    },
                )?;
                cx.global_mut::<Windows>().panel = Some(panel);
                window.hide_window()
            })();
            self.status = match result {
                Ok(()) => "Main window hidden with its state retained.".into(),
                Err(error) => format!("Could not hide the main window: {error}").into(),
            };
            cx.notify();
        }
    }

    impl Render for MainWindow {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            surface()
                .id("main")
                .track_focus(&self.focus)
                .on_action(cx.listener(|this, _: &Increment, _, cx| {
                    this.count += 1;
                    cx.notify();
                }))
                .on_action(cx.listener(Self::show_panel))
                .on_action(|_: &CloseWindow, window, _| { window.hide_window().log_err(); })
                .on_action(|_: &Next, window, cx| window.focus_next(cx))
                .on_action(|_: &Previous, window, cx| window.focus_prev(cx))
                .child(div().text_2xl().child("GPUI Window Lifecycle"))
                .child("Increment the counter, then hide this window. Its identity and state should survive every restore.")
                .child(div().text_xl().child(format!("Counter: {}", self.count)))
                .child(format!("Window: {:?} · Entity: {:?}", window.window_handle().window_id(), cx.entity_id()))
                .child(format!(
                    "Dock/Finder reopen: {} · visible true: {} · visible false: {}",
                    self.visible_reopens + self.hidden_reopens,
                    self.visible_reopens,
                    self.hidden_reopens,
                ))
                .child(format!(
                    "Panel present: {} · panel + visible true: {}",
                    self.panel_reopens, self.visible_panel_reopens,
                ))
                .child(div().p_3().bg(rgb(0xe5eaf0)).rounded_md().child(self.status.clone()))
                .child(div().flex().gap_3()
                    .child(button("increment", "Increment counter", Increment))
                    .child(button("show-panel", "Show panel and hide main", ShowPanel)))
                .child("The red close button and Command-W also hide the main window. Click the Dock icon or open the app in Finder to restore it.")
                .child(button("quit", "Quit example", Quit))
        }
    }

    struct Panel {
        focus: FocusHandle,
        count: usize,
    }

    impl Render for Panel {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            surface()
                .id("panel")
                .track_focus(&self.focus)
                .on_action(|_: &RestoreMain, _, cx| {
                    cx.defer(|cx| { restore_main(None, cx).log_err(); });
                })
                .on_action(|_: &CloseWindow, _, cx| {
                    cx.defer(|cx| { restore_main(None, cx).log_err(); });
                })
                .on_action(|_: &Next, window, cx| window.focus_next(cx))
                .on_action(|_: &Previous, window, cx| window.focus_prev(cx))
                .child(div().text_xl().child("Auxiliary PopUp panel"))
                .child("The main window is hidden. Restore it here, click this app’s Dock icon, or open it in Finder while this panel stays visible.")
                .child(format!("Counter before hiding: {}", self.count))
                .child(button("restore", "Restore main window", RestoreMain))
        }
    }

    fn surface() -> Div {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_6()
            .bg(rgb(0xf7f8fa))
            .text_color(rgb(0x182230))
            .text_sm()
    }

    fn button(id: &'static str, label: &'static str, action: impl Action) -> Stateful<Div> {
        let keyboard_action = action.boxed_clone();
        div()
            .id(id)
            .focusable()
            .tab_stop(true)
            .role(Role::Button)
            .aria_label(label)
            .px_4()
            .py_2()
            .rounded_md()
            .bg(rgb(0x2458a6))
            .text_color(rgb(0xffffff))
            .border_2()
            .border_color(rgb(0x2458a6))
            .hover(|style| style.bg(rgb(0x194582)))
            .focus_visible(|style| style.border_color(rgb(0x182230)))
            .cursor_pointer()
            .child(label)
            .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
            .on_key_down(move |event, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    cx.stop_propagation();
                    window.dispatch_action(keyboard_action.boxed_clone(), cx);
                }
            })
    }

    fn restore_main(request: Option<ReopenRequest>, cx: &mut App) -> gpui::Result<()> {
        // A panel action defers this until its own window update has finished.
        let panel = cx.global_mut::<Windows>().panel.take();
        let panel_present = panel.is_some();
        if let Some(panel) = panel {
            panel.update(cx, |_, window, _| window.remove_window())?;
        }
        if let Some(main) = cx.global::<Windows>().main {
            main.update(cx, |view, window, cx| {
                if let Some(request) = request {
                    if request.has_visible_windows {
                        view.visible_reopens += 1;
                    } else {
                        view.hidden_reopens += 1;
                    }
                    if panel_present {
                        view.panel_reopens += 1;
                        if request.has_visible_windows {
                            view.visible_panel_reopens += 1;
                        }
                    }
                    view.status = format!(
                        "Last Dock/Finder reopen: has_visible_windows = {}; panel_present = {}",
                        request.has_visible_windows, panel_present,
                    )
                    .into();
                } else {
                    view.status = "Restored from the auxiliary panel.".into();
                }
                window.activate_window();
                cx.notify();
            })?;
            cx.activate(true);
        }
        Ok(())
    }

    pub fn run() -> gpui::Result<()> {
        env_logger::init();
        let application = gpui_platform::application();
        application.on_reopen_request(|request, cx| {
            if cx.try_global::<Windows>().is_some() {
                restore_main(Some(request), cx).log_err();
            }
        })?;
        application.run(|cx| {
            cx.set_global(Windows::default());
            cx.bind_keys([
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("cmd-w", CloseWindow, None),
                KeyBinding::new("tab", Next, None),
                KeyBinding::new("shift-tab", Previous, None),
            ]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            let bounds = Bounds::centered(None, size(px(660.), px(640.)), cx);
            let main = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(TitlebarOptions {
                            title: Some("GPUI Window Lifecycle".into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    |window, cx| {
                        window.on_window_should_close(cx, |window, _| {
                            window.hide_window().log_err();
                            false
                        });
                        cx.new(|cx| {
                            let focus = cx.focus_handle();
                            focus.focus(window, cx);
                            MainWindow {
                                focus,
                                count: 0,
                                visible_reopens: 0,
                                hidden_reopens: 0,
                                panel_reopens: 0,
                                visible_panel_reopens: 0,
                                status: "Ready. The main window has not been hidden yet.".into(),
                            }
                        })
                    },
                )
                .expect("open the example's main window");
            cx.global_mut::<Windows>().main = Some(main);
            cx.activate(true);
        });
        Ok(())
    }
}
