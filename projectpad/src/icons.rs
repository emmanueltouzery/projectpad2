#[derive(PartialEq, Debug, Clone)]
pub struct Icon(&'static str);

impl Icon {
    pub fn name(&self) -> &'static str {
        self.0
    }

    pub const REPORTING: Icon = Icon("reporting"); // tv, print, file-invoice, file-pdf, chart-*
    pub const HTTP: Icon = Icon("http");
    pub const WINDOWS: Icon = Icon("windows");
    pub const SERVER: Icon = Icon("server");
    pub const DATABASE: Icon = Icon("database"); // hdd?
    pub const MONITORING: Icon = Icon("monitoring"); // heartbeat?
    pub const NOTE: Icon = Icon("clipboard"); // sticky-note?
    pub const POINT_OF_INTEREST: Icon = Icon("cube"); // cube, file, flag, folder, map_marker_alt?
    pub const SERVER_LINK: Icon = Icon("link"); // link, hdd?
    pub const USER: Icon = Icon("user");
    pub const LOG_FILE: Icon = Icon("log-file");
    pub const CONFIG_FILE: Icon = Icon("config-file");
    pub const COG: Icon = Icon("cog");
    pub const FOLDER_PLUS: Icon = Icon("folder-plus");
    pub const ARCHIVE: Icon = Icon("archive");
    pub const TERMINAL: Icon = Icon("terminal");
}
