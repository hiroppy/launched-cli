use crate::launchd::{self, Service, ServiceSource};
use crate::schedule;
use chrono::Local;
use std::collections::HashMap;

#[derive(Clone, PartialEq)]
pub enum Tab {
    User,
    System,
    All,
    Timeline,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::User, Tab::System, Tab::All, Tab::Timeline]
    }

    pub fn label(&self) -> &str {
        match self {
            Tab::User => "User",
            Tab::System => "System",
            Tab::All => "All",
            Tab::Timeline => "Timeline",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::User => 0,
            Tab::System => 1,
            Tab::All => 2,
            Tab::Timeline => 3,
        }
    }
}

pub struct App {
    pub tab: Tab,
    pub services: Vec<Service>,
    pub cursor: usize,
    pub should_quit: bool,
    pub show_action_menu: bool,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            tab: Tab::User,
            services: Vec::new(),
            cursor: 0,
            should_quit: false,
            show_action_menu: false,
        };
        app.refresh_services();
        app
    }

    pub fn refresh_services(&mut self) {
        let mut services = launchd::scan_plist_dirs();
        let statuses = launchd::query_launchctl_list();
        launchd::merge_status(&mut services, &statuses);
        self.services = services;
    }

    pub fn refresh_status(&mut self) {
        let statuses = launchd::query_launchctl_list();
        launchd::merge_status(&mut self.services, &statuses);
    }

    pub fn filtered_services(&self) -> Vec<&Service> {
        let filtered: Vec<&Service> = match self.tab {
            Tab::User => self
                .services
                .iter()
                .filter(|s| s.source == ServiceSource::UserAgent)
                .collect(),
            Tab::System => self
                .services
                .iter()
                .filter(|s| {
                    s.source == ServiceSource::SystemAgent
                        || s.source == ServiceSource::SystemDaemon
                })
                .collect(),
            Tab::All => self.services.iter().collect(),
            Tab::Timeline => {
                let now = Local::now();
                let mut svcs: Vec<&Service> = self.services.iter().collect();
                svcs.sort_by_key(|s| {
                    schedule::next_run_time(
                        &s.start_calendar_interval,
                        &s.start_interval,
                        now,
                    )
                    .map(|dt| dt.timestamp())
                    .unwrap_or(i64::MAX)
                });
                return svcs;
            }
        };
        filtered
    }

    pub fn grouped_services(&self) -> Vec<(String, Vec<&Service>)> {
        let filtered = self.filtered_services();
        if self.tab == Tab::Timeline {
            return vec![("".to_string(), filtered)];
        }

        let mut groups: Vec<(String, Vec<&Service>)> = Vec::new();
        let mut seen: HashMap<String, usize> = HashMap::new();
        for svc in filtered {
            if let Some(&idx) = seen.get(&svc.category) {
                groups[idx].1.push(svc);
            } else {
                let idx = groups.len();
                seen.insert(svc.category.clone(), idx);
                groups.push((svc.category.clone(), vec![svc]));
            }
        }
        groups.sort_by(|a, b| a.0.cmp(&b.0));
        groups
    }

    pub fn selected_service(&self) -> Option<&Service> {
        let items = self.flat_list();
        items.into_iter().nth(self.cursor)
    }

    pub fn flat_list(&self) -> Vec<&Service> {
        self.grouped_services()
            .into_iter()
            .flat_map(|(_, svcs)| svcs)
            .collect()
    }

    pub fn next_tab(&mut self) {
        self.tab = match self.tab {
            Tab::User => Tab::System,
            Tab::System => Tab::All,
            Tab::All => Tab::Timeline,
            Tab::Timeline => Tab::User,
        };
        self.cursor = 0;
        self.show_action_menu = false;
    }

    pub fn prev_tab(&mut self) {
        self.tab = match self.tab {
            Tab::User => Tab::Timeline,
            Tab::System => Tab::User,
            Tab::All => Tab::System,
            Tab::Timeline => Tab::All,
        };
        self.cursor = 0;
        self.show_action_menu = false;
    }

    pub fn move_cursor_down(&mut self) {
        let len = self.flat_list().len();
        if len > 0 && self.cursor < len - 1 {
            self.cursor += 1;
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }
}
