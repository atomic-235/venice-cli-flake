use crate::api;
use anyhow::Result;
use chrono::DateTime;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row as TuiRow, Table, TableState, Wrap};
use ratatui::Terminal;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

const PRICE_PREF: [&str; 9] = [
    "input",
    "output",
    "generation",
    "cache_input",
    "cache_write",
    "inpaint",
    "per_audio_second",
    "per_second",
    "per_thousand_characters",
];

pub struct Args {
    pub model_type: String,
    pub modality_in: String,
    pub modality_out: String,
    pub vendor: String,
    pub tag: String,
    pub privacy: String,
    pub sort: String,
    pub desc: bool,
    pub search: String,
}

#[derive(Clone)]
struct Comp {
    usd: f64,
    unit: String,
}

#[derive(Clone)]
struct Row {
    id: String,
    name: String,
    mtype: String,
    vendor: String,
    ins: BTreeSet<String>,
    out: String,
    comps: BTreeMap<String, Comp>,
    ctx: f64,
    tags: Vec<String>,
    privacy: String,
    created: f64,
    blob: String,
    raw: Value,
}

pub fn run(args: Args) -> Result<()> {
    let client = api::http(30)?;
    let models = api::fetch_models(&client, "all")?;
    let rows = build_rows(&models);
    let mut app = DashApp::new(rows, args);

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = app.run(&mut terminal);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();
    result
}

fn usd_leaves(value: &Value, path: &str, out: &mut Vec<(String, f64)>) {
    if let Some(object) = value.as_object() {
        if let Some(usd) = object.get("usd").and_then(Value::as_f64) {
            out.push((if path.is_empty() { "?" } else { path }.to_string(), usd));
            return;
        }
        for (key, child) in object {
            let child_path = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            usd_leaves(child, &child_path, out);
        }
    }
}

fn price_unit(path: &str, model_type: &str) -> String {
    let top = path.split('.').next().unwrap_or("");
    let mut unit = match top {
        "input" | "output" | "cache_input" | "cache_write" => "$/Mtok",
        "generation" => "$/gen",
        "inpaint" => "$/op",
        "per_audio_second" => "$/audio-sec",
        "per_second" => "$/sec",
        "per_thousand_characters" => "$/1Kchar",
        "resolutions" | "quality" => "$/img",
        "durations" => "$/gen",
        "upscale" => "$/op",
        "inputImages" => "$/img",
        other => return format!("$/{}", other.replace('_', "-")),
    }
    .to_string();
    if top == "input" && model_type == "tts" {
        unit = "$/Mchar".to_string();
    }
    if let Some((_, rest)) = path.split_once('.') {
        unit.push('·');
        unit.push_str(&rest.replace('.', "·"));
    }
    unit
}

fn price_components(model: &Value) -> BTreeMap<String, Comp> {
    let pricing = &model["model_spec"]["pricing"];
    let model_type = model["type"].as_str().unwrap_or("");
    let mut leaves = Vec::new();
    usd_leaves(pricing, "", &mut leaves);

    let mut comps: BTreeMap<String, Comp> = BTreeMap::new();
    for (path, usd) in leaves {
        let top = path.split('.').next().unwrap_or("").to_string();
        if top == "extended" {
            continue;
        }
        if let Some(entry) = comps.get_mut(&top) {
            if usd < entry.usd {
                *entry = Comp {
                    usd,
                    unit: price_unit(&path, model_type),
                };
            }
        } else {
            comps.insert(
                top,
                Comp {
                    usd,
                    unit: price_unit(&path, model_type),
                },
            );
        }
    }
    comps
}

fn modalities(model: &Value) -> (BTreeSet<String>, String) {
    let model_type = model["type"].as_str().unwrap_or("");
    let spec = &model["model_spec"];
    let caps = &spec["capabilities"];
    let constraints = &spec["constraints"];

    let mut ins = BTreeSet::from(["text".to_string()]);
    if let Some(object) = caps.as_object() {
        for (key, value) in object {
            if value.as_bool() == Some(true) {
                if let Some(rest) = key
                    .strip_prefix("supports")
                    .and_then(|key| key.strip_suffix("Input"))
                {
                    ins.insert(rest.to_lowercase());
                }
            }
        }
    }
    if caps["supportsVision"].as_bool() == Some(true)
        || caps["supportsMultipleImages"].as_bool() == Some(true)
    {
        ins.insert("image".to_string());
    }
    if spec["voice_cloning"].as_bool() == Some(true)
        || spec["supports_custom_voice_id"].as_bool() == Some(true)
    {
        ins.insert("audio".to_string());
    }

    let mut out = None;
    if let Some(model_type_constraint) = constraints["model_type"].as_str() {
        if let Some((source, destination)) = model_type_constraint.split_once("-to-") {
            for part in source
                .to_lowercase()
                .split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|part| !part.is_empty())
            {
                ins.insert(part.to_string());
            }
            let cleaned: String = destination
                .to_lowercase()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect();
            if !cleaned.is_empty() {
                out = Some(cleaned);
            }
        }
    }

    let out = out.unwrap_or_else(|| match model_type {
        "asr" => "text".into(),
        "tts" => "audio".into(),
        "embedding" => "vector".into(),
        "inpaint" | "upscale" => "image".into(),
        "" => "?".into(),
        other => other.into(),
    });

    if model_type == "asr" {
        ins = BTreeSet::from(["audio".to_string()]);
    }
    if model_type == "inpaint" || model_type == "upscale" {
        ins.insert("image".to_string());
    }
    (ins, out)
}

fn vendor_of(model: &Value) -> String {
    let spec = &model["model_spec"];
    let source = spec["modelSource"].as_str().unwrap_or("");
    for prefix in [
        "https://huggingface.co/",
        "http://huggingface.co/",
        "https://www.huggingface.co/",
    ] {
        if let Some(rest) = source.strip_prefix(prefix) {
            if let Some((org, _)) = rest.split_once('/') {
                if org.to_lowercase() != "datasets" {
                    return org.to_lowercase();
                }
            }
        }
    }

    let id = model["id"].as_str().unwrap_or("").to_lowercase();
    let id = id.strip_prefix("e2ee-").unwrap_or(&id);
    if let Some(token) = id
        .split(|c: char| !c.is_ascii_alphanumeric())
        .find(|token| !token.is_empty())
    {
        return token.to_string();
    }

    for prefix in ["https://", "http://"] {
        if let Some(rest) = source.strip_prefix(prefix) {
            let host = rest
                .split(|c: char| c == '/' || c == '?' || c == '#')
                .next()
                .unwrap_or("");
            let host = host.strip_prefix("www.").unwrap_or(host);
            let parts: Vec<&str> = host.split('.').filter(|part| !part.is_empty()).collect();
            if parts.len() >= 2 {
                let tld = parts[parts.len() - 1];
                let name = if ["com", "org", "net", "io", "ai", "dev"].contains(&tld) {
                    parts[parts.len() - 2]
                } else {
                    parts[0]
                };
                return name.to_string();
            }
        }
    }
    "other".to_string()
}

fn tags_of(model: &Value) -> Vec<String> {
    let spec = &model["model_spec"];
    let mut tags = Vec::new();
    if let Some(object) = spec.as_object() {
        for (key, value) in object {
            if value.as_bool() == Some(true) {
                tags.push(key.clone());
            }
        }
    }
    if spec.get("deprecation").is_some_and(|value| !value.is_null()) {
        tags.push("deprecated".to_string());
    }
    if spec["capabilities"]["supportsE2EE"].as_bool() == Some(true) {
        tags.push("e2ee".to_string());
    } else if spec["capabilities"]["supportsTeeAttestation"].as_bool() == Some(true) {
        tags.push("tee".to_string());
    }
    if let Some(sets) = spec["model_sets"].as_array() {
        for set in sets {
            if let Some(set) = set.as_str() {
                tags.push(format!("set:{set}"));
            }
        }
    }
    tags
}

fn fmt_tokens(value: f64) -> String {
    if value <= 0.0 {
        return String::new();
    }
    if value >= 1_000_000.0 {
        let millions = value / 1_000_000.0;
        if (millions - millions.round()).abs() < 0.05 {
            format!("{millions:.0}M")
        } else {
            format!("{millions:.1}M")
        }
    } else if value >= 1000.0 {
        format!("{}K", (value / 1000.0) as i64)
    } else {
        format!("{}", value as i64)
    }
}

fn fmt_price(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let text = format!("{value:.6}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn build_rows(models: &[Value]) -> Vec<Row> {
    models
        .iter()
        .map(|model| {
            let spec = &model["model_spec"];
            let (ins, out) = modalities(model);
            let ctx = spec["availableContextTokens"]
                .as_f64()
                .or_else(|| model["context_length"].as_f64())
                .unwrap_or(0.0);
            Row {
                id: model["id"].as_str().unwrap_or("?").to_string(),
                name: spec["name"].as_str().unwrap_or("").to_string(),
                mtype: model["type"].as_str().unwrap_or("?").to_string(),
                vendor: vendor_of(model),
                ins,
                out,
                comps: price_components(model),
                ctx,
                tags: tags_of(model),
                privacy: spec["privacy"].as_str().unwrap_or("?").to_string(),
                created: model["created"].as_f64().unwrap_or(0.0),
                blob: serde_json::to_string(model)
                    .unwrap_or_default()
                    .to_lowercase(),
                raw: model.clone(),
            }
        })
        .collect()
}

fn unique(values: impl Iterator<Item = String>, head: &str) -> Vec<String> {
    let mut set = BTreeSet::from([head.to_string()]);
    set.extend(values);
    set.into_iter().collect()
}

fn price_keys(rows: &[Row]) -> Vec<String> {
    let mut keys: Vec<String> = PRICE_PREF
        .iter()
        .filter(|key| rows.iter().any(|row| row.comps.contains_key(**key)))
        .map(|key| key.to_string())
        .collect();
    let mut extra: BTreeSet<String> = BTreeSet::new();
    for row in rows {
        for key in row.comps.keys() {
            if !keys.iter().any(|existing| existing == key) {
                extra.insert(key.clone());
            }
        }
    }
    keys.extend(extra);
    keys
}

struct DashApp {
    rows: Vec<Row>,
    type_opts: Vec<String>,
    in_opts: Vec<String>,
    out_opts: Vec<String>,
    vendor_opts: Vec<String>,
    tag_opts: Vec<String>,
    privacy_opts: Vec<String>,
    filters: [String; 6],
    filter_field: usize,
    sort: String,
    reverse: bool,
    search: String,
    input_mode: bool,
    selected: usize,
    detail: bool,
}

impl DashApp {
    fn new(rows: Vec<Row>, args: Args) -> Self {
        let type_opts = unique(rows.iter().map(|row| row.mtype.clone()), "all");
        let in_opts = unique(rows.iter().flat_map(|row| row.ins.iter().cloned()), "any");
        let out_opts = unique(rows.iter().map(|row| row.out.clone()), "any");
        let vendor_opts = unique(rows.iter().map(|row| row.vendor.clone()), "all");
        let tag_opts = unique(rows.iter().flat_map(|row| row.tags.iter().cloned()), "all");
        let privacy_opts = unique(rows.iter().map(|row| row.privacy.clone()), "all");

        let keys = price_keys(&rows);
        let mut sort_keys = keys.clone();
        for key in ["name", "type", "vendor", "ctx", "created"] {
            sort_keys.push(key.to_string());
        }
        let sort = if sort_keys.contains(&args.sort) {
            args.sort.clone()
        } else if args.sort == "price" {
            keys.first().cloned().unwrap_or_else(|| "name".into())
        } else {
            sort_keys.first().cloned().unwrap_or_else(|| "name".into())
        };

        Self {
            rows,
            type_opts,
            in_opts,
            out_opts,
            vendor_opts,
            tag_opts,
            privacy_opts,
            filters: [
                args.model_type,
                args.modality_in,
                args.modality_out,
                args.vendor,
                args.tag,
                args.privacy,
            ],
            filter_field: 0,
            sort,
            reverse: args.desc,
            search: args.search.trim().to_lowercase(),
            input_mode: false,
            selected: 0,
            detail: true,
        }
    }

    fn filter_options(&self, field: usize) -> &[String] {
        match field {
            0 => &self.type_opts,
            1 => &self.in_opts,
            2 => &self.out_opts,
            3 => &self.vendor_opts,
            4 => &self.tag_opts,
            _ => &self.privacy_opts,
        }
    }

    fn visible(&self) -> Vec<usize> {
        let mut visible: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| self.filters[0] == "all" || row.mtype == self.filters[0])
            .filter(|(_, row)| self.filters[1] == "any" || row.ins.contains(&self.filters[1]))
            .filter(|(_, row)| self.filters[2] == "any" || row.out == self.filters[2])
            .filter(|(_, row)| self.filters[3] == "all" || row.vendor == self.filters[3])
            .filter(|(_, row)| self.filters[4] == "all" || row.tags.contains(&self.filters[4]))
            .filter(|(_, row)| self.filters[5] == "all" || row.privacy == self.filters[5])
            .filter(|(_, row)| self.search.is_empty() || row.blob.contains(&self.search))
            .map(|(index, _)| index)
            .collect();

        visible.sort_by(|&a, &b| {
            let left = &self.rows[a];
            let right = &self.rows[b];
            let order = match self.sort.as_str() {
                "ctx" => left.ctx.partial_cmp(&right.ctx).unwrap_or(Ordering::Equal),
                "created" => left
                    .created
                    .partial_cmp(&right.created)
                    .unwrap_or(Ordering::Equal),
                "name" => left.id.to_lowercase().cmp(&right.id.to_lowercase()),
                "type" => left.mtype.cmp(&right.mtype),
                "vendor" => left.vendor.cmp(&right.vendor),
                field => {
                    let left = left
                        .comps
                        .get(field)
                        .map(|comp| comp.usd)
                        .unwrap_or(f64::INFINITY);
                    let right = right
                        .comps
                        .get(field)
                        .map(|comp| comp.usd)
                        .unwrap_or(f64::INFINITY);
                    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
                }
            };
            if self.reverse {
                order.reverse()
            } else {
                order
            }
        });
        visible
    }

    fn cycle_filter(&mut self, delta: i32) {
        let options = self.filter_options(self.filter_field).to_vec();
        let current = &self.filters[self.filter_field];
        let index = options
            .iter()
            .position(|option| option == current)
            .unwrap_or(0) as i32;
        let len = options.len() as i32;
        let next = ((index + delta) % len + len) % len;
        self.filters[self.filter_field] = options[next as usize].clone();
    }

    fn cycle_sort(&mut self) {
        let mut sort_keys = price_keys(&self.rows);
        for key in ["name", "type", "vendor", "ctx", "created"] {
            sort_keys.push(key.to_string());
        }
        let index = sort_keys
            .iter()
            .position(|key| key == &self.sort)
            .unwrap_or(0);
        self.sort = sort_keys[(index + 1) % sort_keys.len()].clone();
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        if self.input_mode {
            match code {
                KeyCode::Esc | KeyCode::Enter => self.input_mode = false,
                KeyCode::Backspace => {
                    self.search.pop();
                }
                KeyCode::Char(character) if !modifiers.contains(KeyModifiers::CONTROL) => {
                    self.search.push(character.to_ascii_lowercase());
                }
                _ => {}
            }
            return false;
        }

        match code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('/') => self.input_mode = true,
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected = self.selected.saturating_add(1)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1)
            }
            KeyCode::Char('g') | KeyCode::Home => self.selected = 0,
            KeyCode::Char('G') | KeyCode::End => self.selected = usize::MAX,
            KeyCode::Char('d') => self.detail = !self.detail,
            KeyCode::Char('r') => self.reverse = !self.reverse,
            KeyCode::Char('s') => self.cycle_sort(),
            KeyCode::Char('b') | KeyCode::Tab => {
                self.filter_field = (self.filter_field + 1) % 6;
            }
            KeyCode::Char('h') | KeyCode::Left => self.cycle_filter(-1),
            KeyCode::Char('l') | KeyCode::Right => self.cycle_filter(1),
            KeyCode::Esc => self.search.clear(),
            _ => {}
        }
        false
    }

    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        loop {
            let visible = self.visible();
            self.selected = self.selected.min(visible.len().saturating_sub(1));
            terminal.draw(|frame| self.draw(frame, &visible))?;

            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press
                        && self.handle_key(key.code, key.modifiers)
                    {
                        return Ok(());
                    }
                }
            }
        }
    }

    fn draw(&self, frame: &mut ratatui::Frame, visible: &[usize]) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(if self.detail { 10 } else { 0 }),
            Constraint::Length(1),
        ])
        .split(area);

        let fields = ["type", "in", "out", "vendor", "tag", "privacy"];
        let mut status = String::new();
        for (index, field) in fields.iter().enumerate() {
            let marker = if index == self.filter_field { "*" } else { "" };
            status.push_str(&format!("{field}:{}{marker}  ", self.filters[index]));
        }
        let arrow = if self.reverse { "▼" } else { "▲" };
        status.push_str(&format!("sort:{} {arrow}  ", self.sort));
        if self.input_mode {
            status.push_str(&format!("search:{}_", self.search));
        } else if !self.search.is_empty() {
            status.push_str(&format!("search:{}  ", self.search));
        }
        status.push_str(&format!("{}/{}", visible.len(), self.rows.len()));

        frame.render_widget(
            Paragraph::new(status).block(Block::default().borders(Borders::ALL)),
            chunks[0],
        );

        let keys = price_keys(&self.rows);
        let mut headers: Vec<String> = vec!["Model".into(), "Type".into(), "Vendor".into()];
        let mut constraints = vec![
            Constraint::Length(34),
            Constraint::Length(8),
            Constraint::Length(12),
        ];
        for key in &keys {
            let unit = self
                .rows
                .iter()
                .find_map(|row| row.comps.get(key).map(|comp| comp.unit.clone()))
                .unwrap_or_default();
            let label = if unit.is_empty() {
                key.clone()
            } else {
                format!("{key} {unit}")
            };
            headers.push(label);
            constraints.push(Constraint::Length(12));
        }
        headers.push("Ctx".into());
        headers.push("Tags".into());
        constraints.push(Constraint::Length(6));
        constraints.push(Constraint::Length(24));

        let rows: Vec<TuiRow> = visible
            .iter()
            .map(|&index| {
                let row = &self.rows[index];
                let mut cells = vec![
                    Cell::from(row.id.clone()),
                    Cell::from(row.mtype.clone()),
                    Cell::from(row.vendor.clone()),
                ];
                for key in &keys {
                    cells.push(Cell::from(
                        row.comps
                            .get(key)
                            .map(|comp| fmt_price(comp.usd))
                            .unwrap_or_else(|| "—".to_string()),
                    ));
                }
                cells.push(Cell::from(fmt_tokens(row.ctx)));
                cells.push(Cell::from(row.tags.join(",")));
                TuiRow::new(cells)
            })
            .collect();

        let table = Table::new(rows, constraints)
            .header(
                TuiRow::new(headers.into_iter().map(Cell::from).collect::<Vec<_>>())
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = TableState::default();
        if !visible.is_empty() {
            state.select(Some(self.selected));
        }
        frame.render_stateful_widget(table, chunks[1], &mut state);

        if self.detail {
            let detail = visible
                .get(self.selected)
                .map(|&index| self.detail_text(&self.rows[index]))
                .unwrap_or_else(|| "no selection".to_string());
            frame.render_widget(
                Paragraph::new(detail)
                    .block(Block::default().borders(Borders::TOP))
                    .wrap(Wrap { trim: false }),
                chunks[2],
            );
        }

        frame.render_widget(
            Paragraph::new(
                "q quit  / search  j/k move  g/G top/bottom  b field  h/l value  s sort  r reverse  d detail",
            )
            .style(Style::default().add_modifier(Modifier::DIM)),
            chunks[3],
        );
    }

    fn detail_text(&self, row: &Row) -> String {
        let spec = &row.raw["model_spec"];
        let mut lines = Vec::new();
        lines.push(format!("{}  {}", row.name, row.id));
        lines.push(format!(
            "type:{}  vendor:{}  modality:{}→{}  privacy:{}  tags:{}",
            row.mtype,
            row.vendor,
            row.ins.iter().cloned().collect::<Vec<_>>().join("+"),
            row.out,
            row.privacy,
            row.tags.join(",")
        ));
        let mut bits = Vec::new();
        if row.ctx > 0.0 {
            bits.push(format!("ctx:{}", fmt_tokens(row.ctx)));
        }
        if spec["maxCompletionTokens"].as_f64().unwrap_or(0.0) > 0.0 {
            bits.push(format!(
                "maxout:{}",
                fmt_tokens(spec["maxCompletionTokens"].as_f64().unwrap_or(0.0))
            ));
        }
        if row.created > 0.0 {
            if let Some(date) = DateTime::from_timestamp(row.created as i64, 0) {
                bits.push(format!("created:{}", date.format("%Y-%m-%d")));
            }
        }
        if let Some(source) = spec["modelSource"].as_str() {
            bits.push(format!("src:{source}"));
        }
        lines.push(bits.join("  "));
        for key in ["pricing", "capabilities", "constraints"] {
            if !spec[key].is_null() {
                lines.push(format!(
                    "{key}: {}",
                    serde_json::to_string(&spec[key]).unwrap_or_default()
                ));
            }
        }
        if let Some(description) = spec["description"].as_str() {
            lines.push(description.trim().to_string());
        }
        lines.join("\n")
    }
}
