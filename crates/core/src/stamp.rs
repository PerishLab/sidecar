pub const FLAG: &str = "--sidecar-stamp";
pub const VERSION: u8 = 1;

pub mod default {
    pub const NAMESPACE: &str = "default";
    pub const MODE: &str = "dev";
    pub const SOURCE: &str = "tool:sidecar";
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stamp {
    pub version: u8,
    pub app: String,
    pub namespace: String,
    pub mode: String,
    pub source: String,
}

impl Stamp {
    pub fn args(&self) -> Vec<String> {
        vec![format!("{FLAG}={}", encode(self))]
    }
}

pub fn flag(args: &[String]) -> Option<String> {
    let prefix = format!("{FLAG}=");
    for (index, value) in args.iter().enumerate() {
        if value == FLAG {
            return args.get(index + 1).cloned();
        }
        if let Some(stripped) = value.strip_prefix(&prefix) {
            return Some(stripped.to_string());
        }
    }
    None
}

pub fn find(args: &[String]) -> Option<Stamp> {
    flag(args).and_then(|value| decode(&value).ok())
}

pub fn encode(stamp: &Stamp) -> String {
    format!(
        "v={};a={};n={};m={};s={}",
        stamp.version,
        percent::encode(&stamp.app),
        percent::encode(&stamp.namespace),
        percent::encode(&stamp.mode),
        percent::encode(&stamp.source),
    )
}

pub fn decode(value: &str) -> Result<Stamp, String> {
    let mut version = None;
    let mut app = None;
    let mut namespace = None;
    let mut mode = None;
    let mut source = None;

    for part in value.split(';') {
        let Some((key, raw)) = part.split_once('=') else {
            return Err("stamp segment must use key=value form".to_string());
        };
        let decoded = percent::decode(raw)?;
        match key {
            "v" if version.is_none() => version = Some(vetted(&decoded)?),
            "a" if app.is_none() => app = Some(decoded),
            "n" if namespace.is_none() => namespace = Some(decoded),
            "m" if mode.is_none() => mode = Some(decoded),
            "s" if source.is_none() => source = Some(decoded),
            "v" | "a" | "n" | "m" | "s" => {
                return Err(format!("duplicate stamp key {key:?}"));
            }
            other => return Err(format!("unknown stamp key {other:?}")),
        }
    }

    Ok(Stamp {
        version: required(version, "v")?,
        app: required(app, "a")?,
        namespace: required(namespace, "n")?,
        mode: required(mode, "m")?,
        source: required(source, "s")?,
    })
}

fn required<T>(value: Option<T>, key: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("stamp missing {key:?}"))
}

fn vetted(decoded: &str) -> Result<u8, String> {
    let parsed = decoded
        .parse::<u8>()
        .map_err(|_| "stamp version must be an integer".to_string())?;
    if parsed != VERSION {
        return Err(format!("unsupported stamp version {parsed}"));
    }
    Ok(parsed)
}

pub(crate) mod percent {
    pub(crate) fn encode(value: &str) -> String {
        let mut encoded = String::new();
        for byte in value.bytes() {
            if unreserved(byte) {
                encoded.push(byte as char);
            } else {
                encoded.push('%');
                encoded.push(hex::encode(byte >> 4));
                encoded.push(hex::encode(byte & 0x0f));
            }
        }
        encoded
    }

    pub(crate) fn decode(value: &str) -> Result<String, String> {
        let bytes = value.as_bytes();
        let mut decoded = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'%' => {
                    let Some(high) = bytes.get(index + 1).and_then(|byte| hex::decode(*byte))
                    else {
                        return Err("stamp value contains invalid percent escape".to_string());
                    };
                    let Some(low) = bytes.get(index + 2).and_then(|byte| hex::decode(*byte)) else {
                        return Err("stamp value contains invalid percent escape".to_string());
                    };
                    decoded.push((high << 4) | low);
                    index += 3;
                }
                byte => {
                    decoded.push(byte);
                    index += 1;
                }
            }
        }
        String::from_utf8(decoded).map_err(|_| "stamp value is not valid UTF-8".to_string())
    }

    fn unreserved(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_')
    }

    mod hex {
        pub(crate) fn encode(value: u8) -> char {
            match value {
                0..=9 => (b'0' + value) as char,
                10..=15 => (b'A' + value - 10) as char,
                _ => unreachable!(),
            }
        }

        pub(crate) fn decode(byte: u8) -> Option<u8> {
            match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                b'A'..=b'F' => Some(byte - b'A' + 10),
                _ => None,
            }
        }
    }
}
