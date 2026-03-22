use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, Padding, Paragraph, Tabs, Widget},
};

use crate::models::hotspot::Hotspot;

#[derive(Debug, Clone, Copy)]
pub enum EvidenceTab {
    Commits,
    CoChange,
    Ownership,
    Notes,
}

impl EvidenceTab {
    pub fn switch_left(self) -> Self {
        match self {
            EvidenceTab::Commits => EvidenceTab::Notes,
            EvidenceTab::CoChange => EvidenceTab::Commits,
            EvidenceTab::Ownership => EvidenceTab::CoChange,
            EvidenceTab::Notes => EvidenceTab::Ownership,
        }
    }

    pub fn switch_right(self) -> Self {
        match self {
            EvidenceTab::Commits => EvidenceTab::CoChange,
            EvidenceTab::CoChange => EvidenceTab::Ownership,
            EvidenceTab::Ownership => EvidenceTab::Notes,
            EvidenceTab::Notes => EvidenceTab::Commits,
        }
    }

    pub fn index(self) -> usize {
        match self {
            EvidenceTab::Commits => 0,
            EvidenceTab::CoChange => 1,
            EvidenceTab::Ownership => 2,
            EvidenceTab::Notes => 3,
        }
    }
}

#[derive(Debug)]
pub struct EvidenceWidget {
    active_tab: EvidenceTab,
    selected_hotspot_name: String,
    commits_lines: Vec<String>,
    co_change_lines: Vec<String>,
    ownership_lines: Vec<String>,
    notes_lines: Vec<String>,
}

impl EvidenceWidget {
    pub fn new(active_tab: EvidenceTab, hotspot: Option<&Hotspot>) -> Self {
        let mut widget = EvidenceWidget {
            active_tab,
            selected_hotspot_name: "[none]".to_string(),
            commits_lines: vec!["No hotspot selected.".to_string()],
            co_change_lines: vec!["No hotspot selected.".to_string()],
            ownership_lines: vec!["No hotspot selected.".to_string()],
            notes_lines: vec!["No hotspot selected.".to_string()],
        };
        widget.set_selected_hotspot(hotspot);
        widget
    }

    pub fn set_active_tab(&mut self, active_tab: EvidenceTab) {
        self.active_tab = active_tab;
    }

    pub fn set_selected_hotspot(&mut self, hotspot: Option<&Hotspot>) {
        if let Some(hotspot) = hotspot {
            self.selected_hotspot_name = hotspot.location().to_string();
            self.commits_lines = hotspot.commit_evidence_lines();
            self.co_change_lines = hotspot.co_change_evidence_lines();
            self.ownership_lines = hotspot.ownership_evidence_lines();
            self.notes_lines = hotspot.notes_evidence_lines();
            return;
        }

        self.selected_hotspot_name = "[none]".to_string();
        self.commits_lines = vec!["No hotspot selected.".to_string()];
        self.co_change_lines = vec!["No hotspot selected.".to_string()];
        self.ownership_lines = vec!["No hotspot selected.".to_string()];
        self.notes_lines = vec!["No hotspot selected.".to_string()];
    }

    fn active_tab_lines(&self) -> &[String] {
        match self.active_tab {
            EvidenceTab::Commits => &self.commits_lines,
            EvidenceTab::CoChange => &self.co_change_lines,
            EvidenceTab::Ownership => &self.ownership_lines,
            EvidenceTab::Notes => &self.notes_lines,
        }
    }

    fn active_tab_heading(&self) -> &'static str {
        match self.active_tab {
            EvidenceTab::Commits => "Recent commits and authors:",
            EvidenceTab::CoChange => "Top coupled files:",
            EvidenceTab::Ownership => "Ownership distribution:",
            EvidenceTab::Notes => "Risk explanation:",
        }
    }

    fn active_tab_text(&self) -> Text<'static> {
        let mut lines = vec![
            Line::from(format!("Selected hotspot: {}", self.selected_hotspot_name)).bold(),
            Line::from(self.active_tab_heading()),
        ];

        for item in self.active_tab_lines() {
            lines.push(Line::from(format!("- {}", item)));
        }

        Text::from(lines)
    }

    fn block(&self) -> Block<'_> {
        Block::bordered()
            .title(Line::from(" Evidence / Explanation ".bold()).centered())
            .border_set(border::PLAIN)
            .padding(Padding::horizontal(1))
    }
}

impl Widget for &EvidenceWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
            .split(inner);

        let tabs = Tabs::new(vec!["Commits", "Co-change", "Ownership", "Notes"])
            .select(self.active_tab.index())
            .highlight_style(Style::default().blue().bold())
            .divider("|");
        Widget::render(tabs, layout[0], buf);
        Widget::render(Paragraph::new(self.active_tab_text()), layout[1], buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::test_support::{hotspot_payment, hotspot_ui};

    fn render_lines(widget: &EvidenceWidget, width: u16, height: u16) -> Vec<String> {
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        Widget::render(widget, buf.area, &mut buf);
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .collect()
    }

    #[test]
    fn render_snapshot_commits_tab_with_hotspot() {
        let hotspot = hotspot_payment();
        let widget = EvidenceWidget::new(EvidenceTab::Commits, Some(&hotspot));

        let rendered = render_lines(&widget, 100, 12);

        let expected = vec![
            "┌───────────────────────────────────── Evidence / Explanation ─────────────────────────────────────┐",
            "│  Commits | Co-change | Ownership | Notes                                                         │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "│ Selected hotspot: src/service/payment.rs                                                         │",
            "│ Recent commits and authors:                                                                      │",
            "│ - abcdef0 Alice - 2026-03-01 - 19 lines - Refine payment retries                                 │",
            "│ - 1234567 Bob - 2026-02-27 - 12 lines - Split gateway adapter                                    │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "└──────────────────────────────────────────────────────────────────────────────────────────────────┘",
        ];
        assert_eq!(rendered, expected);
    }

    #[test]
    fn render_snapshot_co_change_tab_with_hotspot() {
        let hotspot = hotspot_payment();
        let widget = EvidenceWidget::new(EvidenceTab::CoChange, Some(&hotspot));

        let rendered = render_lines(&widget, 100, 12);

        let expected = vec![
            "┌───────────────────────────────────── Evidence / Explanation ─────────────────────────────────────┐",
            "│  Commits | Co-change | Ownership | Notes                                                         │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "│ Selected hotspot: src/service/payment.rs                                                         │",
            "│ Top coupled files:                                                                               │",
            "│ - src/lib.rs (8 commits together)                                                                │",
            "│ - src/main.rs (3 commits together)                                                               │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "└──────────────────────────────────────────────────────────────────────────────────────────────────┘",
        ];
        assert_eq!(rendered, expected);
    }

    #[test]
    fn render_snapshot_ownership_tab_with_hotspot() {
        let hotspot = hotspot_payment();
        let widget = EvidenceWidget::new(EvidenceTab::Ownership, Some(&hotspot));

        let rendered = render_lines(&widget, 100, 12);

        let expected = vec![
            "┌───────────────────────────────────── Evidence / Explanation ─────────────────────────────────────┐",
            "│  Commits | Co-change | Ownership | Notes                                                         │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "│ Selected hotspot: src/service/payment.rs                                                         │",
            "│ Ownership distribution:                                                                          │",
            "│ - Alice <alice@example.com> - 6 touches (60%)                                                    │",
            "│ - Bob <bob@example.com> - 4 touches (40%)                                                        │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "└──────────────────────────────────────────────────────────────────────────────────────────────────┘",
        ];
        assert_eq!(rendered, expected);
    }

    #[test]
    fn render_snapshot_notes_tab_with_hotspot() {
        let hotspot = hotspot_payment();
        let widget = EvidenceWidget::new(EvidenceTab::Notes, Some(&hotspot));

        let rendered = render_lines(&widget, 100, 12);

        let expected = vec![
            "┌───────────────────────────────────── Evidence / Explanation ─────────────────────────────────────┐",
            "│  Commits | Co-change | Ownership | Notes                                                         │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "│ Selected hotspot: src/service/payment.rs                                                         │",
            "│ Risk explanation:                                                                                │",
            "│ - High churn risk: frequent line movement suggests structural pressure.                          │",
            "│ - Ownership is distributed; refactoring can reduce coordination overhead.                        │",
            "│ - Recent activity spike: changes are active now, making this a timely candidate.                 │",
            "│                                                                                                  │",
            "│                                                                                                  │",
            "└──────────────────────────────────────────────────────────────────────────────────────────────────┘",
        ];
        assert_eq!(rendered, expected);
    }

    #[test]
    fn render_snapshot_without_hotspot() {
        let widget = EvidenceWidget::new(EvidenceTab::Commits, None);

        let rendered = render_lines(&widget, 88, 10);

        let expected = vec![
            "┌─────────────────────────────── Evidence / Explanation ───────────────────────────────┐",
            "│  Commits | Co-change | Ownership | Notes                                             │",
            "│                                                                                      │",
            "│                                                                                      │",
            "│ Selected hotspot: [none]                                                             │",
            "│ Recent commits and authors:                                                          │",
            "│ - No hotspot selected.                                                               │",
            "│                                                                                      │",
            "│                                                                                      │",
            "└──────────────────────────────────────────────────────────────────────────────────────┘",
        ];
        assert_eq!(rendered, expected);
    }

    #[test]
    fn set_active_tab_changes_rendered_content() {
        let hotspot = hotspot_payment();
        let mut widget = EvidenceWidget::new(EvidenceTab::Commits, Some(&hotspot));

        let commits_render = render_lines(&widget, 90, 10).join("\n");
        widget.set_active_tab(EvidenceTab::CoChange);
        let co_change_render = render_lines(&widget, 90, 10).join("\n");

        assert!(commits_render.contains("Recent commits and authors:"));
        assert!(co_change_render.contains("Top coupled files:"));
    }

    #[test]
    fn set_selected_hotspot_updates_all_tab_datasets() {
        let hotspot_a = hotspot_payment();
        let hotspot_b = hotspot_ui();
        let mut widget = EvidenceWidget::new(EvidenceTab::Commits, Some(&hotspot_a));

        widget.set_selected_hotspot(Some(&hotspot_b));

        widget.set_active_tab(EvidenceTab::Commits);
        let commits_render = render_lines(&widget, 90, 10).join("\n");
        assert!(commits_render.contains("Selected hotspot: src/ui/render.rs"));
        assert!(commits_render.contains("Tune viewport math"));

        widget.set_active_tab(EvidenceTab::CoChange);
        let co_change_render = render_lines(&widget, 90, 10).join("\n");
        assert!(co_change_render.contains("src/ui/layout.rs (2 commits together)"));

        widget.set_active_tab(EvidenceTab::Ownership);
        let ownership_render = render_lines(&widget, 90, 10).join("\n");
        assert!(ownership_render.contains("Eve <eve@example.com> - 5 touches (100%)"));
    }

    #[test]
    fn evidence_tab_switch_wraps_left_and_right() {
        assert!(matches!(
            EvidenceTab::Commits.switch_left(),
            EvidenceTab::Notes
        ));
        assert!(matches!(
            EvidenceTab::Notes.switch_right(),
            EvidenceTab::Commits
        ));
        assert_eq!(EvidenceTab::Commits.index(), 0);
        assert_eq!(EvidenceTab::CoChange.index(), 1);
        assert_eq!(EvidenceTab::Ownership.index(), 2);
        assert_eq!(EvidenceTab::Notes.index(), 3);
    }
}
