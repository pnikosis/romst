use std::{collections::{HashMap, HashSet}, iter::FromIterator, rc::Rc};

use anyhow::Result;
use log::{debug, error};
use rusqlite::{Connection, params};

use crate::{data::{models::{file::DataFile, game::Game}, reader::sqlite::DBReader}};
use super::DataWriter;

#[derive(Debug)]
pub struct IdsCounter {
    rom: u32,
    disk: u32,
}

impl IdsCounter {
    pub fn new() -> Self { Self { rom: 0, disk: 0 } }
    pub fn get_next_rom(&mut self) -> u32 {
        let id = self.rom;
        self.rom += 1;
        id
    }
}

#[derive(Debug)]
pub struct DBWriter<'d> {
    conn: &'d mut Connection,
    ids: IdsCounter,
    buffer: Buffer,
    buffer_size: u16,
}

#[derive(Debug)]
struct Buffer {
    ids: IdsCounter,
    games: HashMap<String, Rc<Game>>,

    roms: HashMap<DataFile, u32>,
    game_roms: HashMap<String, Vec<(u32, String)>>,
    samples: HashMap<String, HashSet<String>>,
    device_refs: HashMap<String, Vec<String>>,
}

impl Buffer {
    fn new() -> Self { Self {
        ids: IdsCounter::new(),
        games: HashMap::new(),
        roms: HashMap::new(), 
        game_roms: HashMap::new(),
        samples: HashMap::new(),
        device_refs: HashMap::new() }
    }

    fn len(&self) -> usize {
        self.games.len() + self.samples.len()
    }

    fn add_game(&mut self, game_name: String, game: Rc<Game>) {
        self.games.insert(game_name, game);
    }

    fn add_roms(&mut self, roms: Vec<DataFile>) -> Vec<(u32, DataFile)> {
        let mut rom_ids = vec![];
        roms.into_iter().for_each(|rom| {
            match self.roms.get(&rom) {
                Some(rom_id) => {
                    rom_ids.push((*rom_id, rom));
                }
                None => {
                    let id = self.ids.get_next_rom();
                    self.roms.insert(rom.clone(), id);
                    rom_ids.push((id, rom));
                }
            }
        });

        rom_ids
    }

    fn add_roms_for_game(&mut self, game_name: String, rom_ids: Vec<(u32, String)>) {
        self.game_roms.insert(game_name, rom_ids);
    }

    fn add_sample_pack(&mut self, sample_pack: String, samples: Vec<String>) {
        self.samples.entry(sample_pack).or_insert(HashSet::new()).extend(samples);
    }

    fn add_device_refs(&mut self, game_name: String, device_refs: Vec<String>) {
        self.device_refs.insert(game_name, device_refs);
    }
}

impl <'d> DBWriter<'d> {
    pub fn from_connection(conn: &'d mut Connection, buffer_size: u16) -> Self {
        Self { conn, ids: IdsCounter::new(), buffer: Buffer::new(), buffer_size }
    }

    fn remove_table_if_exist(&self, table_name: &str) -> Result<()> {
        let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name = ?1;";
        let result: Result<String, rusqlite::Error>  = self.conn.query_row(sql, params![ table_name ], |row| {
            Ok(row.get(0)?)
        });

        match result {
            Ok(name) => {
                debug!("Deleting table {}...", name);
                let sql_drop = format!("DROP TABLE IF EXISTS {};", name);
                self.conn.execute(&sql_drop, params![ ])?;
                Ok(())
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => { Ok(()) }
            Err(e) => { Err(e.into()) }
        }
    }

    fn create_schema(&self) -> Result<()> {
        self.create_table_info()?;
        self.create_table_roms()?;
        self.create_table_games()?;
        self.create_table_game_roms()?;
        self.create_table_device_refs()?;
        self.create_table_disks()?;
        self.create_table_game_disks()?;
        self.create_table_samples()?;
        //self.create_table_game_samples()?;

        Ok(())
    }

    fn create_table_info(&self) -> Result<()> {
        self.remove_table_if_exist("info")?;
        self.conn.execute(
            "CREATE TABLE info (
                name        TEXT,
                description TEXT,
                version     TEXT);", 
            params![])?;

        Ok(())
    }

    fn create_table_roms(&self) -> Result<()> {
        debug!("Creating ROMS table");
        self.remove_table_if_exist("roms")?;
        // Rom
        self.conn.execute(
            "CREATE TABLE roms (
                id      INTEGER PRIMARY KEY,
                sha1    TEXT,
                md5     TEXT,
                crc     TEXT,
                size    INT,
                status  TEXT);", 
            params![])?;
        debug!("Creating ROMS indexes");
        // Indexes
        self.conn.execute( "CREATE INDEX roms_sha1 ON roms(sha1);", params![])?;
        self.conn.execute( "CREATE INDEX roms_md5 ON roms(md5);", params![])?;
        self.conn.execute( "CREATE INDEX roms_crc ON roms(crc);", params![])?;
        self.conn.execute( "CREATE INDEX roms_checks ON roms(sha1, md5, crc);", params![])?;

        Ok(())
    }

    fn create_table_games(&self) -> Result<()> {
        debug!("Creating Games table");
        self.remove_table_if_exist("games")?;
        // Machines/Games
        self.conn.execute(
            "CREATE TABLE games (
                name        TEXT PRIMARY KEY,
                clone_of    TEXT,
                rom_of      TEXT,
                source_file TEXT,
                sample_of   TEXT,
                info_desc   TEXT,
                info_year   TEXT,
                info_manuf  TEXT);",
            params![])?;
        debug!("Creating Games indexes");
        // Indexes
        self.conn.execute( "CREATE INDEX games_parents ON games(clone_of);", params![])?;
        self.conn.execute( "CREATE INDEX games_samples ON games(sample_of);", params![])?;

        Ok(())
    }

    fn create_table_game_roms(&self) -> Result<()> {
        debug!("Creating Games/ROMs table");
        self.remove_table_if_exist("game_roms")?;
        // Machine/Roms
        self.conn.execute(
            "CREATE TABLE game_roms (
                game_name   TEXT,
                rom_id      INTEGER,
                name        TEXT,
                parent      TEXT,
                PRIMARY KEY (game_name, rom_id, name));",
            params![])?;
        debug!("Creating Games/ROMs indexes");
        // Indexes
        self.conn.execute( "CREATE INDEX game_roms_game ON game_roms(game_name);", params![])?;
        self.conn.execute( "CREATE INDEX game_roms_rom ON game_roms(rom_id);", params![])?;
        self.conn.execute( "CREATE INDEX game_roms_parents ON game_roms(parent);", params![])?;

        Ok(())
    }

    fn create_table_device_refs(&self) -> Result<()> {
        debug!("Creating device_refs table");
        self.remove_table_if_exist("devices")?;
        // Machine/Roms
        self.conn.execute(
            "CREATE TABLE devices (
                game_name   TEXT,
                device_ref  TEXT,
                PRIMARY KEY (game_name, device_ref));",
            params![])?;
        debug!("Creating devices indexes");
        // Indexes
        self.conn.execute( "CREATE INDEX devices_games ON devices(game_name);", params![])?;
        self.conn.execute( "CREATE INDEX devices_refs ON devices(device_ref);", params![])?;

        Ok(())
    }

    fn create_table_disks(&self) -> Result<()> {
        debug!("Creating disks table");
        self.remove_table_if_exist("disks")?;
        // Rom
        self.conn.execute(
            "CREATE TABLE disks (
                id      INTEGER PRIMARY KEY,
                sha1    TEXT,
                region  TEXT,
                status  TEXT);", 
            params![])?;
        debug!("Creating disks indexes");
        // Indexes
        self.conn.execute( "CREATE INDEX disks_sha1 ON disks(sha1);", params![])?;

        Ok(())
    }

    fn create_table_game_disks(&self) -> Result <()> {
        debug!("Creating Games/Disks table");
        self.remove_table_if_exist("game_disks")?;
        // Machine/Roms
        self.conn.execute(
            "CREATE TABLE game_disks (
                game_name   TEXT,
                disk_id     TEXT,
                parent      TEXT,
                PRIMARY KEY (game_name, disk_id));",
            params![])?;
        debug!("Creating Games/Disks indexes");
        // Indexes
        self.conn.execute( "CREATE INDEX game_disks_game ON game_disks(game_name);", params![])?;
        self.conn.execute( "CREATE INDEX game_disks_disks ON game_disks(disk_id);", params![])?;

        Ok(())
    }

    fn create_table_samples(&self) -> Result<()> {
        debug!("Creating samples table");
        self.remove_table_if_exist("samples")?;
        // Rom
        self.conn.execute(
            "CREATE TABLE samples (
                sample_set  TEXT,
                sample      TEXT,
                PRIMARY KEY (sample_set, sample));", 
            params![])?;
        debug!("Creating samples indexes");
        // Indexes
        self.conn.execute( "CREATE INDEX sample_sets ON samples(sample_set);", params![])?;

        Ok(())
    }

    /*fn create_table_game_samples(&self) -> Result <()> {
        debug!("Creating Games/Samples table");
        self.remove_table_if_exist("game_samples")?;
        // Machine/Roms
        self.conn.execute(
            "CREATE TABLE game_samples (
                game_name   TEXT,
                sample_set  TEXT,
                PRIMARY KEY (game_name, sample_set));",
            params![])?;
        debug!("Creating Games/Samples indexes");
        // Indexes
        self.conn.execute( "CREATE INDEX game_samples_game ON game_samples(game_name);", params![])?;
        self.conn.execute( "CREATE INDEX game_samples_sample ON game_samples(sample_set);", params![])?;

        Ok(())
    }*/

    fn get_rom_ids(&mut self, roms: Vec<DataFile>) -> Result<Vec<(u32, String)>> {
        // We search the database
        let rom_ids = DBReader::get_ids_from_files(self.conn, roms)?;
        let mut rom_name_pair: Vec<(u32, String)> = rom_ids.found.iter().map(|rom|{
            (rom.0, rom.1.name.clone())
        }).collect();

        // We add in the buffer what is not in the database
        let mut in_buffer: Vec<(u32, String)> = self.buffer.add_roms(rom_ids.not_found).into_iter().map(|rom| {
            (rom.0, rom.1.name)
        }).collect();
        rom_name_pair.append(&mut in_buffer);

        // We remove the duplicates
        rom_name_pair.sort();
        rom_name_pair.dedup();
        // println!("ROMS {:?}", rom_name_pair);
        Ok(rom_name_pair)
    }

    fn write_buffer(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        let game_buffer = &self.buffer.games;
        let rom_buffer = &self.buffer.roms;
        let game_rom_buffer = &self.buffer.game_roms;
        let sample_buffer = &self.buffer.samples;

        let values = game_buffer.values();
        for value in values {
            let game = value;
            let p = params![game.name,
                game.clone_of,
                game.rom_of,
                game.source_file,
                game.info_description,
                game.info_year,
                game.info_manufacturer];
            match tx.execute("INSERT INTO games (name, clone_of, rom_of, source_file, info_desc, info_year, info_manuf)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
                p) {
                    Ok(_) => {}
                    Err(e) => { error!("Error inserting row in the games db: {}", e) }
                }
        }

        for rom_data in rom_buffer {
            let rom_row_id = rom_data.1;
            let rom = rom_data.0;
            tx.execute(
                "INSERT INTO roms (id, sha1, md5, crc, size, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                params![ rom_row_id, rom.sha1, rom.md5, rom.crc, rom.size, rom.status ])?;
        }

        for game_roms in game_rom_buffer {
            let game_name = game_roms.0;
            let rom_id_names = game_roms.1;
            for rom_id_name in rom_id_names {
                let result = tx.execute(
                    "INSERT INTO game_roms (game_name, rom_id, name) VALUES (?1, ?2, ?3);",
                    params![ game_name, rom_id_name.0, rom_id_name.1 ] );
                match result {
                    Ok(_n) => { debug!("Inserted rom {} with id {} to the game {}", rom_id_name.1, rom_id_name.0, game_name) }
                    Err(e) => { error!("Error adding rom `{}` to the game {}: {}", rom_id_name.1, "", e) }
                }
            }
        }

        for sample_sets in sample_buffer {
            let sample_set = sample_sets.0;
            let samples = sample_sets.1;
            for sample in samples {
                let result = tx.execute(
                    "INSERT OR IGNORE INTO samples (sample_set, sample) VALUES (?1, ?2);", 
                    params![sample_set, sample]);
                match result {
                    Ok(_n) => { debug!("Inserted sample `{}` for sample set `{}`", sample, sample_set) }
                    Err(e) => { error!("Error inserting sample `{}` for sample set `{}`: {}", sample, sample_set, e) }
                }
            }
        }

        tx.commit()?;
        self.buffer.games.clear();
        self.buffer.roms.clear();
        self.buffer.game_roms.clear();
        self.buffer.samples.clear();

        Ok(())
    }

    fn get_roms_from_parents(&mut self) -> Result<Vec<(String, u32, String)>>{
        let mut stmt = self.conn.prepare("SELECT games.name AS game_name, game_roms.rom_id, game_roms.game_name as parent, game_roms.name FROM game_roms
            JOIN games ON games.clone_of = game_roms.game_name
            WHERE games.clone_of IS NOT NULL;")?;
        let rows = stmt.query_map(params![], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?.filter_map(|row| row.ok());

        Ok(Vec::from_iter(rows))
    }

    fn add_game(&mut self, game: Rc<Game>) -> Result<()> {
        let game_name = game.name.to_owned();
        self.buffer.add_game(game_name, game); 

        if self.buffer.len() as u16 >= self.buffer_size {
            self.write_buffer()?;
        };

        Ok(())
    }

    fn add_roms_for_game(&mut self, roms: Vec<DataFile>, game_name: &str) -> Result<()> {
        let rom_list = self.get_rom_ids(roms)?;

        self.buffer.add_roms_for_game(game_name.to_string(), rom_list);

        Ok(())
    }

    fn add_samples(&mut self, samples: Vec<String>, sample_pack: &str) -> Result<()> {
        self.buffer.add_sample_pack(sample_pack.to_string(), samples);

        Ok(())
    }
}

impl <'d> DataWriter for DBWriter<'d> {
    fn init(&self) -> Result<()> {
        self.create_schema()
    }
    
    fn on_new_entry(&mut self, game: Game, roms: Vec<DataFile>, disks: Vec<DataFile>, samples: Vec<String>, device_refs: Vec<String>) -> Result<()> {
        let game_ref = Rc::new(game);

        let game_name = &game_ref.name;
        let sample = game_ref.sample_of.as_ref();

        self.add_game(Rc::clone(&game_ref))?;
        self.add_roms_for_game(roms, game_name)?;
        //self.add_disks_for_game(disks, game_name)?;
        if let Some(sample_name) = sample {
            self.add_samples(samples, sample_name)?;
        }
        //self.add_devices_for_game(device_refs, game_name)?;

        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.write_buffer()?;
        let roms_from_parents = self.get_roms_from_parents()?;

        let tx = self.conn.transaction()?;
        for item in roms_from_parents {
            let game_name = item.0;
            let rom_id = item.1;
            let parent = item.2;

            let result = tx.execute("UPDATE game_roms SET parent = ?1
                WHERE game_roms.game_name = ?2 AND game_roms.rom_id = ?3;", params![parent, game_name, rom_id])?;
            if result > 1 {
                debug!("Updated {} rows, should be only 1 for game {}, rom_id {}, with parent {}, unless is a 'nodump'", result, game_name, rom_id, parent);
            }
        }
        tx.commit()?;

        Ok(())
    }

}