use super::super::widgets::evidence::{EvidenceTab, EvidenceWidget};
use super::super::widgets::hotspot_detail::HotspotDetailWidget;
use super::super::widgets::hotspots::Hotspots;
use super::View;
use crate::glitzer::repo::RepositoryAccess;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    symbols::border,
    widgets::Block,
};

#[derive(Debug)]
pub struct MainView {
    hotspots: Hotspots,
    selected_hotspot_index: usize,
    active_tab: EvidenceTab,
    hotspot_detail: HotspotDetailWidget,
    evidence: EvidenceWidget,
}

impl MainView {
    pub fn new(repo: &impl RepositoryAccess) -> Result<Self> {
        let hotspots = Hotspots::new(repo)?;
        let hotspot_detail = HotspotDetailWidget::from_hotspot(hotspots.selected_hotspot());
        let selected_name = hotspots
            .selected_hotspot()
            .map(|hotspot| hotspot.location().to_string())
            .unwrap_or("[none]".to_string());
        let evidence = EvidenceWidget::new(EvidenceTab::Commits, selected_name);

        Ok(MainView {
            hotspots,
            selected_hotspot_index: 0,
            active_tab: EvidenceTab::Commits,
            hotspot_detail,
            evidence,
        })
    }

    fn switch_tab_left(&mut self) {
        self.active_tab = self.active_tab.switch_left();
        self.evidence.set_active_tab(self.active_tab);
    }

    fn switch_tab_right(&mut self) {
        self.active_tab = self.active_tab.switch_right();
        self.evidence.set_active_tab(self.active_tab);
    }

    fn refresh_selection_widgets(&mut self) {
        self.hotspot_detail = HotspotDetailWidget::from_hotspot(self.hotspots.selected_hotspot());

        let selected_name = self
            .hotspots
            .selected_hotspot()
            .map(|hotspot| hotspot.location().to_string())
            .unwrap_or("[none]".to_string());

        self.evidence.set_selected_hotspot_name(selected_name);
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
        frame.render_widget(&self.hotspot_detail, top_layout[1]);
        frame.render_widget(&self.evidence, outer_layout[1]);
    }

    fn handle_input(&mut self, input: KeyEvent) {
        match input.code {
            KeyCode::Char('q') => {
                std::process::exit(0);
            }
            KeyCode::Char('j') => {
                if self.hotspots.len() > 0 {
                    self.selected_hotspot_index = (self.selected_hotspot_index + 1)
                        .min(self.hotspots.len().saturating_sub(1));
                    self.hotspots
                        .set_selected_index(self.selected_hotspot_index);
                    self.refresh_selection_widgets();
                }
            }
            KeyCode::Char('k') => {
                self.selected_hotspot_index = self.selected_hotspot_index.saturating_sub(1);
                self.hotspots
                    .set_selected_index(self.selected_hotspot_index);
                self.refresh_selection_widgets();
            }
            KeyCode::Char('h') => self.switch_tab_left(),
            KeyCode::Char('l') => self.switch_tab_right(),
            _ => {}
        }
    }
}
