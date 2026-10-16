use std::fs;
use std::io::ErrorKind;

use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::sd::mmc::{SdMmcHostConfiguration, SdMmcHostDriver, SDMMC1};
use esp_idf_svc::hal::sd::{SdCardConfiguration, SdCardDriver};
use esp_idf_svc::io::vfs::MountedFatfs;
use vision_core::people::{People, PeopleError};

use crate::board::SdPins;

const MOUNT: &str = "/sdcard";
const PEOPLE: &str = "/sdcard/people.bin";
const PEOPLE_NEW: &str = "/sdcard/people.new";
const SNAPSHOTS: &str = "/sdcard/people";

type Card = MountedFatfs<Fatfs<SdCardDriver<SdMmcHostDriver<'static>>>>;

pub struct Storage {
    _card: Card,
}

// The card handle holds raw driver pointers. It is only kept alive here and never touched again; file access
// goes through the VFS, which is thread-safe.
unsafe impl Send for Storage {}

impl Storage {
    pub fn mount(slot: SDMMC1<'static>, pins: SdPins) -> anyhow::Result<Self> {
        let host = SdMmcHostDriver::new_1bit(
            slot,
            pins.cmd,
            pins.clk,
            pins.d0,
            None::<AnyIOPin>,
            None::<AnyIOPin>,
            &SdMmcHostConfiguration::new(),
        )?;
        let card = SdCardDriver::new_mmc(host, &SdCardConfiguration::new())?;
        let card = MountedFatfs::mount(Fatfs::new_sdcard(0, card)?, MOUNT, 4)?;
        match fs::create_dir(SNAPSHOTS) {
            Err(error) if error.kind() != ErrorKind::AlreadyExists => return Err(error.into()),
            _ => {}
        }
        Ok(Self { _card: card })
    }

    pub fn load_people(&self, embedding_len: usize) -> anyhow::Result<People> {
        let read = fs::read(PEOPLE).or_else(|error| match error.kind() {
            ErrorKind::NotFound => fs::read(PEOPLE_NEW),
            _ => Err(error),
        });
        match read {
            Ok(bytes) => match People::decode(&bytes) {
                Ok(people) if people.embedding_len() == embedding_len => Ok(people),
                Ok(_) => anyhow::bail!("{PEOPLE} was enrolled with a different face model"),
                Err(PeopleError::Corrupt) => anyhow::bail!("{PEOPLE} is corrupt"),
                Err(error) => anyhow::bail!("{PEOPLE}: {error:?}"),
            },
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(People::new(embedding_len)),
            Err(error) => Err(error.into()),
        }
    }

    pub fn save_people(&self, people: &People) -> anyhow::Result<()> {
        fs::write(PEOPLE_NEW, people.encode())?;
        match fs::remove_file(PEOPLE) {
            Err(error) if error.kind() != ErrorKind::NotFound => return Err(error.into()),
            _ => {}
        }
        fs::rename(PEOPLE_NEW, PEOPLE)?;
        Ok(())
    }

    pub fn save_snapshot(&self, id: u16, jpeg: &[u8]) -> anyhow::Result<()> {
        fs::write(snapshot_path(id), jpeg)?;
        Ok(())
    }

    pub fn remove_snapshot(&self, id: u16) {
        let _ = fs::remove_file(snapshot_path(id));
    }
}

pub fn load_snapshot(id: u16) -> Option<Vec<u8>> {
    fs::read(snapshot_path(id)).ok()
}

fn snapshot_path(id: u16) -> String {
    format!("{SNAPSHOTS}/{id}.jpg")
}
