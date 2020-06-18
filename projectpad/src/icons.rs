#[derive(PartialEq, Debug, Clone)]
pub struct Icon(&'static str);

impl Icon {
    pub fn name(&self) -> &'static str {
        self.0
    }

    pub const SERVER: Icon = Icon("server");
    pub const NOTE: Icon = Icon("clipboard"); // sticky-note?
    pub const POINT_OF_INTEREST: Icon = Icon("cube"); // cube, file, flag, folder, map_marker_alt?
    pub const SERVER_LINK: Icon = Icon("link"); // link, hdd?
}
