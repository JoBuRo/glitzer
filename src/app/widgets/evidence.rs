use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, Padding, Paragraph, Tabs, Widget},
};

use super::hotspots::Hotspot;

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

    fn block(&self) -> Block {
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
