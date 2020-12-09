use crate::{sysout::SysOutWriterReporter, data::models::{file::DataFile, game::Game}};
use anyhow::Result;

use super::DataWriter;

#[derive(Debug)]
pub struct SysOutWriter {
    reporter: SysOutWriterReporter,
}

impl SysOutWriter {
    pub fn new() -> Self { Self { reporter: SysOutWriterReporter::new() } }
}

impl DataWriter for SysOutWriter {
    fn init(&self) -> Result<()> {
        // println!("Initializing...");
        Ok(())
    }

    fn on_new_game(&mut self, game: Game) -> Result<()> {
        self.reporter.current_game(&game.name);
        Ok(())
    }

    fn on_new_roms(&mut self, _game: Game, roms: Vec<DataFile>) -> Result<()> {
        for rom in roms {
            self.reporter.current_rom(&rom.name);
        };
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}
