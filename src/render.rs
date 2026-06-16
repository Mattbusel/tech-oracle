//! Render the public (delayed) page from the revealed archive using minijinja.
//! The template file is embedded at compile time, so the binary stays
//! self-contained. This module knows nothing about payments — it only renders
//! whatever the caller decided is public, plus static subscribe links.

use crate::model::Prediction;

#[allow(clippy::too_many_arguments)]
pub fn render(
    generated_human: &str,
    reveal_delay_days: i64,
    featured_date_human: &str,
    featured: &[Prediction],
    archive: &[Prediction], // already newest-first
    payment_link: &str,
    portal_url: &str,
    early_access_url: &str,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(crate::OUT_DIR)?;

    let tmpl_src = include_str!("../templates/index.html");
    let mut env = minijinja::Environment::new();
    env.add_template("index", tmpl_src)?;
    let tmpl = env.get_template("index")?;

    let html = tmpl.render(minijinja::context! {
        generated_human => generated_human,
        reveal_delay_days => reveal_delay_days,
        featured_date_human => featured_date_human,
        featured => featured,
        archive => archive,
        total => archive.len(),
        payment_link => payment_link,
        portal_url => portal_url,
        early_access_url => early_access_url,
    })?;

    std::fs::write(crate::OUT_HTML, html)?;
    Ok(())
}
