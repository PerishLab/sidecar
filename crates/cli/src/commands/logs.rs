use super::*;
use std::io::{Read, Seek, SeekFrom};

struct Seat {
    name: String,
    path: PathBuf,
}

pub(super) fn logs(
    paths: &Paths,
    targets: &[&Target],
    follow: bool,
    lines: Option<usize>,
) -> Result<(), String> {
    let seats: Vec<Seat> = targets
        .iter()
        .map(|target| Seat {
            name: target.name.clone(),
            path: super::runtime::state::log(paths, &target.name),
        })
        .collect();
    if follow {
        return trail(&seats);
    }
    for seat in &seats {
        show(seat, lines, seats.len() > 1);
    }
    Ok(())
}

fn show(seat: &Seat, lines: Option<usize>, prefixed: bool) {
    let Ok(text) = fs::read_to_string(&seat.path) else {
        println!("- {}: no log at {}", seat.name, seat.path.display());
        return;
    };
    let held: Vec<&str> = text.lines().collect();
    let start = lines.map_or(0, |count| held.len().saturating_sub(count));
    for line in &held[start..] {
        say(&seat.name, line, prefixed);
    }
}

fn trail(seats: &[Seat]) -> Result<(), String> {
    let prefixed = seats.len() > 1;
    let mut marks: Vec<u64> = seats.iter().map(|seat| length(&seat.path)).collect();
    loop {
        for (index, seat) in seats.iter().enumerate() {
            let size = length(&seat.path);
            if size < marks[index] {
                marks[index] = 0;
            }
            if size > marks[index] {
                marks[index] = drain(seat, marks[index], prefixed);
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn drain(seat: &Seat, from: u64, prefixed: bool) -> u64 {
    let Ok(mut file) = fs::File::open(&seat.path) else {
        return from;
    };
    if file.seek(SeekFrom::Start(from)).is_err() {
        return from;
    }
    let mut text = String::new();
    if file.read_to_string(&mut text).is_err() {
        return from;
    }
    for line in text.lines() {
        say(&seat.name, line, prefixed);
    }
    from + text.len() as u64
}

fn length(path: &Path) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn say(name: &str, line: &str, prefixed: bool) {
    if prefixed {
        println!("{name} | {line}");
    } else {
        println!("{line}");
    }
}
