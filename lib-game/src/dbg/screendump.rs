use std::fmt;
use std::sync::{LazyLock, Mutex};

use egui::{CollapsingHeader, Ui, Window};
use hashbrown::HashMap;
use mimiq::egui;

const DUMP_LINE_CAPACITY: usize = 255;
const DUMP_CAPACITY: usize = 100;

pub static GLOBAL_DUMP: LazyLock<ScreenDump> = LazyLock::new(ScreenDump::new);

pub struct ScreenDump(Mutex<ScreenDumpBuff>);

impl ScreenDump {
    pub fn new() -> Self {
        ScreenDump(Mutex::new(ScreenDumpBuff::new()))
    }

    pub fn put_line(&self, mod_str: &'static str, args: fmt::Arguments) {
        let mut buff = self.0.lock().expect("Dangling mutex");
        if buff.locked {
            return;
        }

        let line = buff.get_next_line(mod_str);
        fmt::write(line, args).expect("failed to write a line");
    }

    pub fn lock(&self) {
        let mut buff = self.0.lock().expect("Dangling mutex");
        buff.locked = true;
    }

    pub fn reset(&self) {
        let mut buff = self.0.lock().expect("Dangling mutex");
        buff.reset();
    }

    pub fn ui(&self, ctx: &egui::Context) {
        let buff = self.0.lock().expect("Dangling mutex");
        Window::new("Value dump").show(ctx, |ui| buff.ui(ui));
    }
}

impl Default for ScreenDump {
    fn default() -> Self {
        ScreenDump::new()
    }
}

struct ScreenDumpBuff {
    locked: bool,
    lines_by_mod: HashMap<&'static str, ScreenDumpModEntry>,
}

impl ScreenDumpBuff {
    fn new() -> Self {
        ScreenDumpBuff { locked: false, lines_by_mod: HashMap::new() }
    }

    fn get_next_line(&mut self, mod_str: &'static str) -> &mut String {
        let entry = self.lines_by_mod.entry(mod_str).or_default();
        entry.get_next_line()
    }

    fn reset(&mut self) {
        self.lines_by_mod
            .values_mut()
            .for_each(ScreenDumpModEntry::reset);
        self.locked = false;
    }

    fn ui(&self, ui: &mut Ui) {
        for (mod_str, entry) in &self.lines_by_mod {
            CollapsingHeader::new(*mod_str)
                .default_open(false)
                .show(ui, |ui| entry.ui(ui));
        }
    }
}

struct ScreenDumpModEntry {
    next_line: usize,
    lines: [String; DUMP_CAPACITY],
}

impl ScreenDumpModEntry {
    fn new() -> ScreenDumpModEntry {
        ScreenDumpModEntry {
            next_line: 0,
            lines: std::array::from_fn(|_| String::with_capacity(DUMP_LINE_CAPACITY)),
        }
    }

    fn reset(&mut self) {
        self.next_line = 0;
        self.lines.iter_mut().for_each(String::clear);
    }

    fn get_next_line(&mut self) -> &mut String {
        debug_assert!(self.next_line <= DUMP_CAPACITY);
        let res = &mut self.lines[self.next_line];
        self.next_line += 1;
        res
    }

    fn ui(&self, ui: &mut Ui) {
        for line in &self.lines[0..self.next_line] {
            ui.label(line);
        }
    }
}

impl Default for ScreenDumpModEntry {
    fn default() -> Self {
        ScreenDumpModEntry::new()
    }
}
