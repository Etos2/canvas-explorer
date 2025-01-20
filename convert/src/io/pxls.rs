use std::{fmt::Display, io::BufRead, str::FromStr};

use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, NaiveDateTime};

pub const DATE_FMT: &str = "%Y-%m-%d %H:%M:%S,%3f";

#[derive(Debug, Clone, PartialEq)]
pub enum PxlsAction {
    Place,
    Undo,
    Rollback,
    RollbackUndo,
    Overwrite,
    Nuke,
}

impl Display for PxlsAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            PxlsAction::Place => "user place",
            PxlsAction::Undo => "user undo",
            PxlsAction::Rollback => "mod overwrite",
            PxlsAction::RollbackUndo => "rollback undo",
            PxlsAction::Overwrite => "rollback",
            PxlsAction::Nuke => "console nuke",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PxlsUserId {
    Sha256(String),
    Username(String),
}

impl PxlsUserId {
    pub fn as_str(&self) -> &str {
        match self {
            PxlsUserId::Sha256(id) => id.as_str(),
            PxlsUserId::Username(id) => id.as_str(),
        }
    }
}

impl Display for PxlsUserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PxlsLine {
    pub(crate) time: i64,
    pub(crate) pos: (u32, u32),
    pub(crate) index: u16,
    pub(crate) action: PxlsAction,
    pub(crate) id: PxlsUserId,
}

impl Display for PxlsLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let time_fmt = DateTime::from_timestamp_millis(self.time)
            .unwrap()
            .format(DATE_FMT);
        write!(
            f,
            "{}\t{}\t{}\t{}\t{}\t{}",
            time_fmt, self.id, self.pos.0, self.pos.1, self.index, self.action
        )
    }
}

impl PxlsLine {
    fn parse_bytes(data: &[u8]) -> Result<Self> {
        let mut bytes = data.split(|&b| b == b'\n' || b == b'\t');
        let time = read_time(bytes.next().ok_or_else(|| anyhow!("unexpected eof"))?)?;
        let id = read_userid(bytes.next().ok_or_else(|| anyhow!("unexpected eof"))?)?;
        let x = read_int(bytes.next().ok_or_else(|| anyhow!("unexpected eof"))?)?;
        let y = read_int(bytes.next().ok_or_else(|| anyhow!("unexpected eof"))?)?;
        let index = read_int(bytes.next().ok_or_else(|| anyhow!("unexpected eof"))?)?;
        let action = read_action(bytes.next().ok_or_else(|| anyhow!("unexpected eof"))?)?;

        // Assert bytes.next == None OR bytes.next == Empty
        if let Some(data) = bytes.next() {
            if !data.is_empty() {
                bail!("expected eof or newline (found {data:?})")
            }
        }

        Ok(PxlsLine {
            time,
            pos: (x, y),
            index,
            action,
            id,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PxlsFile {
    lines: Vec<PxlsLine>,
}

impl PxlsFile {
    pub fn read_from(mut rdr: impl BufRead) -> Result<Self> {
        let mut lines = Vec::new();
        loop {
            // TODO: Vec::with_capacity (determine maximum *reasonable* line length)
            let mut dyn_buf = Vec::new();
            if rdr.read_until(b'\n', &mut dyn_buf)? == 0 {
                break;
            }
            lines.push(PxlsLine::parse_bytes(&dyn_buf)?);
        }

        Ok(PxlsFile { lines })
    }

    pub fn iter(&self) -> impl Iterator<Item = &PxlsLine> {
        self.lines.iter()
    }

    pub fn lines(&self) -> &[PxlsLine] {
        self.lines.as_slice()
    }
}

fn read_time(data: &[u8]) -> Result<i64> {
    Ok(
        NaiveDateTime::parse_from_str(std::str::from_utf8(data)?, DATE_FMT)
            .map(|t| t.and_utc().timestamp_millis())?,
    )
}

fn read_userid(data: &[u8]) -> Result<PxlsUserId> {
    let id = String::from_utf8(data.to_vec())?;
    if data.len() == 64 {
        Ok(PxlsUserId::Sha256(id))
    } else {
        Ok(PxlsUserId::Username(id))
    }
}

fn read_int<T>(data: &[u8]) -> Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    Ok(std::str::from_utf8(data)?.parse::<T>()?)
}

fn read_action(data: &[u8]) -> Result<PxlsAction> {
    let action = String::from_utf8(data.to_vec())?;
    Ok(match action.as_str() {
        "user place" => PxlsAction::Place,
        "user undo" => PxlsAction::Undo,
        "mod overwrite" => PxlsAction::Overwrite,
        "rollback undo" => PxlsAction::RollbackUndo,
        "rollback" => PxlsAction::Rollback,
        "console nuke" => PxlsAction::Nuke,
        _ => bail!("invalid action ({action})"),
    })
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_read_line_sha256() {
        let data = b"2021-03-19 08:03:47,016\t982b5ed74632b00ad8d75ae4995117b5f93ac9432fe1a3c73444132fc0e1248f\t773\t1761\t4\tuser undo";
        let output = PxlsLine::parse_bytes(data).unwrap();
        assert_eq!(output, PxlsLine {
            time: 1616141027016,
            pos: (773, 1761),
            index: 4,
            action: PxlsAction::Undo,
            id: PxlsUserId::Sha256(
                "982b5ed74632b00ad8d75ae4995117b5f93ac9432fe1a3c73444132fc0e1248f".to_string()
            ),
        })
    }

    #[test]
    fn test_read_line_username() {
        let data = b"2024-01-21 01:30:47,016\tEtos2\t43\t21\t3\tconsole nuke";
        let output = PxlsLine::parse_bytes(data).unwrap();
        assert_eq!(output, PxlsLine {
            time: 1705800647016,
            pos: (43, 21),
            index: 3,
            action: PxlsAction::Nuke,
            id: PxlsUserId::Username("Etos2".to_string()),
        })
    }

    #[test]
    fn test_read_file() {
        let data = b"2021-03-19 08:03:47,016\t982b5ed74632b00ad8d75ae4995117b5f93ac9432fe1a3c73444132fc0e1248f\t773\t1761\t4\tuser undo\n2024-01-21 01:30:47,016\tEtos2\t43\t21\t3\tconsole nuke";
        let rdr = Cursor::new(data);
        let file = PxlsFile::read_from(rdr).unwrap();
        assert_eq!(file, PxlsFile {
            lines: vec![
                PxlsLine {
                    time: 1616141027016,
                    pos: (773, 1761),
                    index: 4,
                    action: PxlsAction::Undo,
                    id: PxlsUserId::Sha256(
                        "982b5ed74632b00ad8d75ae4995117b5f93ac9432fe1a3c73444132fc0e1248f"
                            .to_string()
                    )
                },
                PxlsLine {
                    time: 1705800647016,
                    pos: (43, 21),
                    index: 3,
                    action: PxlsAction::Nuke,
                    id: PxlsUserId::Username("Etos2".to_string()),
                }
            ]
        });
    }

    #[test]
    fn test_display() {
        let data = b"2021-03-19 08:03:47,016\t982b5ed74632b00ad8d75ae4995117b5f93ac9432fe1a3c73444132fc0e1248f\t773\t1761\t4\tuser undo";
        let output = PxlsLine::parse_bytes(data).unwrap();
        assert_eq!(&output.to_string(), std::str::from_utf8(data).unwrap())
    }
}
