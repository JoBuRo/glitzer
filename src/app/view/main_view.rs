use super::super::widgets::hotspots::Hotspots;
use super::View;
use crate::glitzer::repo::RepositoryAccess;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    symbols::border,
    widgets::{Block, Paragraph, Tabs},
};

#[derive(Debug)]
enum EvidenceTab {
    Commits,
    CoChange,
    Ownership,
    Notes,
}

#[derive(Debug)]
pub struct MainView {
    hotspots: Hotspots,
    selected_hotspot_index: usize,
    active_tab: EvidenceTab,
}

impl MainView {
    pub fn new(repo: &impl RepositoryAccess) -> Result<Self> {
        let hotspots = Hotspots::new(repo)?;

        Ok(MainView {
            hotspots,
            selected_hotspot_index: 0,
            active_tab: EvidenceTab::Commits,
        })
    }

    fn switch_tab_left(&mut self) {
        self.active_tab = match self.active_tab {
            EvidenceTab::Commits => EvidenceTab::Notes,
            EvidenceTab::CoChange => EvidenceTab::Commits,
            EvidenceTab::Ownership => EvidenceTab::CoChange,
            EvidenceTab::Notes => EvidenceTab::Ownership,
        }
    }

    fn switch_tab_right(&mut self) {
        self.active_tab = match self.active_tab {
            EvidenceTab::Commits => EvidenceTab::CoChange,
            EvidenceTab::CoChange => EvidenceTab::Ownership,
            EvidenceTab::Ownership => EvidenceTab::Notes,
            EvidenceTab::Notes => EvidenceTab::Commits,
        }
    }

    fn active_tab_index(&self) -> usize {
        match self.active_tab {
            EvidenceTab::Commits => 0,
            EvidenceTab::CoChange => 1,
            EvidenceTab::Ownership => 2,
            EvidenceTab::Notes => 3,
        }
    }

    fn active_tab_text(&self) -> &'static str {
        match self.active_tab {
            EvidenceTab::Commits => {
                "Placeholder: recent commits and authors for the selected hotspot will appear here."
            }
            EvidenceTab::CoChange => {
                "Placeholder: top coupled files and co-change strength will appear here."
            }
            EvidenceTab::Ownership => {
                "Placeholder: author distribution and ownership concentration will appear here."
            }
            EvidenceTab::Notes => {
                "Placeholder: narrative risk explanation and refactoring notes will appear here."
            }
        }
    }
}

impl View for MainView {
    fn render(&self, frame: &mut Frame) {
        let title = Line::from("  ✨ Glitzer ✨ ".bold());

        let instructions = Line::from(vec![
            " Quit ".into(),
            "<Q> ".blue().bold(),
            " Move ".into(),
            "<J/K> ".blue().bold(),
            " Tabs ".into(),
            "<H/L> ".blue().bold(),
        ]);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        frame.render_widget(block, frame.area());

        let inner_area = frame.area().inner(Margin::new(1, 1));
        let outer_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
            .split(inner_area);
        let top_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(outer_layout[0]);

        frame.render_widget(&self.hotspots, top_layout[0]);

        let detail_title = Line::from("  Selected Hotspot  ".bold());
        let detail_block = Block::bordered()
            .title(detail_title.centered())
            .border_set(border::PLAIN)
            .padding(ratatui::widgets::Padding::horizontal(1));
        let detail_text = Text::from(vec![
            Line::from(format!(
                "Name: [placeholder] hotspot #{}",
                self.selected_hotspot_index + 1
            )),
            Line::from("Score: [placeholder]"),
            Line::from(""),
            Line::from("Why it ranks high:"),
            Line::from("- [placeholder] high churn"),
            Line::from("- [placeholder] many authors"),
            Line::from("- [placeholder] strong co-change"),
            Line::from("- [placeholder] recent activity spike"),
            Line::from(""),
            Line::from("Churn trend / sparkline: [placeholder]"),
        ]);
        frame.render_widget(
            Paragraph::new(detail_text).block(detail_block),
            top_layout[1],
        );

        let evidence_block = Block::bordered()
            .title(Line::from("  Evidence / Explanation  ".bold()).centered())
            .border_set(border::PLAIN)
            .padding(ratatui::widgets::Padding::horizontal(1));

        let evidence_inner = evidence_block.inner(outer_layout[1]);
        frame.render_widget(evidence_block, outer_layout[1]);

        let evidence_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
            .split(evidence_inner);

        let tabs = Tabs::new(vec!["Commits", "Co-change", "Ownership", "Notes"])
            .select(self.active_tab_index())
            .highlight_style(Style::default().blue().bold())
            .divider("|");
        frame.render_widget(tabs, evidence_layout[0]);

        frame.render_widget(Paragraph::new(self.active_tab_text()), evidence_layout[1]);
    }

    fn handle_input(&mut self, input: KeyEvent) {
        match input.code {
            KeyCode::Char('q') => {
                std::process::exit(0);
            }
            KeyCode::Char('j') => {
                self.selected_hotspot_index = self.selected_hotspot_index.saturating_add(1)
            }
            KeyCode::Char('k') => {
                self.selected_hotspot_index = self.selected_hotspot_index.saturating_sub(1)
            }
            KeyCode::Char('h') => self.switch_tab_left(),
            KeyCode::Char('l') => self.switch_tab_right(),
            _ => {}
        }
    }
}
