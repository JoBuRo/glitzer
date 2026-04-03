use std::io;

use color_eyre::eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, read};
use ratatui::Frame;

use crate::models::hotspot_source::HotspotSource;
use crate::ui::view::{View, main_view::MainView};

pub struct App {
    current_view: MainView,
    is_running: bool,
}

impl App {
    pub fn new(source: impl HotspotSource) -> Result<Self> {
        Ok(App {
            current_view: MainView::new(&source)?,
            is_running: true,
        })
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
        while self.is_running {
            terminal.draw(|frame| {
                self.render(frame);
            })?;

            self.handle_events()?;
        }
        Ok(())
    }

    fn quit(&mut self) {
        self.is_running = false
    }

    fn render(&self, frame: &mut Frame) {
        self.current_view.render(frame);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if let Event::Key(key_event) = read()? {
            self.handle_key_event(key_event);
        }

        Ok(())
    }

    fn handle_key_event(&mut self, event: KeyEvent) {
        match (event.code, event.modifiers) {
            (KeyCode::Char('q'), _) => {
                self.quit();
            }
            (KeyCode::Char('j'), _) => self.current_view.switch_selection_down(),
            (KeyCode::Char('k'), _) => self.current_view.switch_selection_up(),
            (KeyCode::Char('h'), _) => self.current_view.switch_tab_left(),
            (KeyCode::Char('l'), _) => self.current_view.switch_tab_right(),
            (KeyCode::Char('d'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                self.current_view.scroll_evidence_down()
            }
            (KeyCode::Char('u'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                self.current_view.scroll_evidence_up()
            }
            _ => {}
        }
    }

    #[cfg(test)]
    fn handle_key_code(&mut self, code: KeyCode) {
        self.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::hotspot::{Hotspot, HotspotTestData};
    use crate::ui::widgets::evidence::EvidenceTab;

    struct MockHotspotSource {
        hotspots: Vec<Hotspot>,
    }

    impl HotspotSource for MockHotspotSource {
        fn hotspots(&self, _max_commits: usize) -> Result<Vec<Hotspot>> {
            Ok(self.hotspots.clone())
        }
    }

    fn hotspot(path: &'static str) -> Hotspot {
        Hotspot::test_fixture(HotspotTestData {
            location: path,
            touches: 3,
            lines_touched: 10,
            recent_points: 2,
            most_recent_days: 1,
            authors: &["test@example.com"],
            recent_commits: vec![("abcdef1", "Test", "2026-01-01", "msg", 4)],
            co_changes: vec![("other.rs", 1)],
            author_touches: vec![("Test <test@example.com>", 3)],
            default_rank_multiplier_percent: 100,
        })
    }

    fn app_with_two_hotspots() -> App {
        let source = MockHotspotSource {
            hotspots: vec![hotspot("a.rs"), hotspot("b.rs")],
        };
        App::new(source).unwrap()
    }

    #[test]
    fn key_q_stops_app_loop() {
        let mut app = app_with_two_hotspots();
        assert!(app.is_running);

        app.handle_key_code(KeyCode::Char('q'));

        assert!(!app.is_running);
    }

    #[test]
    fn key_j_and_k_update_selection() {
        let mut app = app_with_two_hotspots();
        assert_eq!(app.current_view.selected_hotspot_index(), 0);

        app.handle_key_code(KeyCode::Char('j'));
        assert_eq!(app.current_view.selected_hotspot_index(), 1);

        app.handle_key_code(KeyCode::Char('k'));
        assert_eq!(app.current_view.selected_hotspot_index(), 0);
    }

    #[test]
    fn key_h_and_l_switch_tabs() {
        let mut app = app_with_two_hotspots();
        assert!(matches!(
            app.current_view.active_tab(),
            EvidenceTab::Commits
        ));

        app.handle_key_code(KeyCode::Char('l'));
        assert!(matches!(
            app.current_view.active_tab(),
            EvidenceTab::CoChange
        ));

        app.handle_key_code(KeyCode::Char('h'));
        assert!(matches!(
            app.current_view.active_tab(),
            EvidenceTab::Commits
        ));
    }

    #[test]
    fn ctrl_u_and_ctrl_d_scroll_evidence() {
        let mut app = app_with_two_hotspots();
        assert_eq!(app.current_view.evidence_scroll_offset(), 0);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(app.current_view.evidence_scroll_offset() > 0);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(app.current_view.evidence_scroll_offset(), 0);
    }
}
