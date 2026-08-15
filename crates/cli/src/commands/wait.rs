use super::*;
use std::time::Instant;

pub(super) fn wait(target: &Target, grants: &Grants, patience: u64) -> Result<(), String> {
    let url = probe(target, grants)?;
    let deadline = Instant::now() + Duration::from_secs(patience);
    loop {
        if answers(&url) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "`{}` did not answer {url} with a 2xx within {patience}s",
                target.name
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn probe(target: &Target, grants: &Grants) -> Result<String, String> {
    let Some(template) = target.health.as_ref() else {
        return Err(format!(
            "`{}` declares no health_url, so --wait has nothing to poll",
            target.name
        ));
    };
    let url = grants.fill(template, &format!("{} health_url", target.name))?;
    if url.starts_with("https://") {
        return Err(format!(
            "`{}` health_url is https; sidecar polls plain http on the loopback it grants",
            target.name
        ));
    }
    Ok(url)
}

fn answers(url: &str) -> bool {
    ureq::get(url)
        .call()
        .is_ok_and(|response| response.status().is_success())
}
