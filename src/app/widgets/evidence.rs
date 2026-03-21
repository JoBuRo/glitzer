use ratatui::{
    prelude::*,
    symbols::border,
    text::Line,
    widgets::{Block, Padding, Paragraph, Tabs, Widget},
};

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
}

impl EvidenceWidget {
    pub fn new(active_tab: EvidenceTab, selected_hotspot_name: String) -> Self {
        EvidenceWidget {
            active_tab,
            selected_hotspot_name,
        }
    }

    pub fn set_active_tab(&mut self, active_tab: EvidenceTab) {
        self.active_tab = active_tab;
    }

    pub fn set_selected_hotspot_name(&mut self, selected_hotspot_name: String) {
        self.selected_hotspot_name = selected_hotspot_name;
    }

    fn active_tab_text(&self) -> String {
        match self.active_tab {
            EvidenceTab::Commits => format!(
                "Placeholder: recent commits and authors touching {} will appear here.",
                self.selected_hotspot_name
            ),
            EvidenceTab::CoChange => format!(
                "Placeholder: top files that co-change with {} will appear here.",
                self.selected_hotspot_name
            ),
            EvidenceTab::Ownership => format!(
                "Placeholder: author distribution and ownership concentration for {} will appear here.",
                self.selected_hotspot_name
            ),
            EvidenceTab::Notes => format!(
                "Placeholder: narrative risk explanation and refactoring notes for {} will appear here.",
                self.selected_hotspot_name
            ),
        }
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
