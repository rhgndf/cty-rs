use chrono::FixedOffset;
use regex::Regex;
use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{self, BufRead},
    path::Path,
};

#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub cq: u32,
    pub itu: u32,
    pub continent: String,
    pub lat: f32,
    pub lon: f32,
    pub timezone: FixedOffset,
    pub prefix: String,
    pub waedc: bool,
    pub is_exact: bool,
}
impl Default for Entity {
    fn default() -> Self {
        Entity {
            name: String::new(),
            cq: 0,
            itu: 0,
            continent: String::new(),
            lat: 0.0,
            lon: 0.0,
            timezone: FixedOffset::east_opt(0).unwrap(),
            prefix: String::new(),
            waedc: false,
            is_exact: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct Cty {
    pub entities: HashMap<String, Entity>,
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

impl Cty {
    pub fn load(filename: &str) -> Result<Cty, Box<dyn Error>> {
        let mut cty = Cty::default();
        let mut last_entity = Entity::default();
        let lines = read_lines(filename)?;

        let cq_regex = Regex::new("\\((\\d+)\\)")?;
        let itu_regex = Regex::new("\\[(\\d+)\\]")?;
        let latlon_regex = Regex::new("<(.*)/(.*)>")?;
        let continent_regex = Regex::new("\\{(.*)\\}")?;
        let timezone_regex = Regex::new("~(.*)~")?;

        for line in lines {
            let line = line?;
            let parts = line.split(':').map(str::trim).collect::<Vec<&str>>();

            if parts.len() > 2 {
                last_entity = Entity {
                    name: parts[0].to_string(),
                    cq: parts[1].parse::<u32>()?,
                    itu: parts[2].parse::<u32>()?,
                    continent: parts[3].to_string(),
                    lat: parts[4].parse::<f32>()?,
                    lon: parts[5].parse::<f32>()?,
                    timezone: FixedOffset::east_opt(0).ok_or("Invalid timezone")?,
                    prefix: parts[7].to_string().trim_start_matches("*").to_string(),
                    waedc: parts[7].starts_with('*'),
                    is_exact: false,
                };
                cty.entities
                    .insert(last_entity.prefix.clone(), last_entity.clone());
            } else {
                let aliases = line
                    .trim_end_matches(';')
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(str::trim)
                    .collect::<Vec<&str>>();

                for alias in aliases {
                    let is_exact = alias.starts_with('=');
                    let alias = alias.trim_start_matches('=');
                    // Get the string until end of string of one of the following characters is found: ([#~
                    let pos = alias
                        .find(|c| c == '(' || c == '[' || c == '#' || c == '~')
                        .unwrap_or(alias.len());
                    let override_alias = &alias[..pos];
                    let overrides = &alias[pos..];
                    let mut entity = last_entity.clone();
                    entity.is_exact = is_exact;
                    // Match by (.*)
                    let cq_override = cq_regex.captures(overrides);
                    if cq_override.is_some() {
                        entity.cq = cq_override.unwrap()[1].parse::<u32>()?;
                    }
                    // Match by [.*]
                    let itu_override = itu_regex.captures(overrides);
                    if itu_override.is_some() {
                        entity.itu = itu_override.unwrap()[1].parse::<u32>()?;
                    }
                    // Match by <.*/.*>
                    let latlon_override = latlon_regex.captures(overrides);
                    if latlon_override.is_some() {
                        let latlon = latlon_override.unwrap();
                        entity.lat = latlon[1].parse::<f32>()?;
                        entity.lon = latlon[2].parse::<f32>()?;
                    }
                    // Match by {.*}
                    let continent_override = continent_regex.captures(overrides);
                    if continent_override.is_some() {
                        entity.continent = continent_override.unwrap()[1].to_string();
                    }
                    // Match by ~.*~
                    let timezone_override = timezone_regex.captures(overrides);
                    if timezone_override.is_some() {
                        entity.timezone = FixedOffset::east_opt(
                            timezone_override.unwrap()[1].parse::<i32>()? * 3600,
                        )
                        .ok_or("Invalid timezone")?;
                    }
                    cty.entities.insert(override_alias.to_string(), entity);
                }
            }
        }
        Ok(cty)
    }
    pub fn lookup(&self, callsign: &str) -> Option<&Entity> {
        self.entities
            .get(callsign)
            .filter(|e| e.is_exact)
            .or((1..=callsign.len())
                .rev()
                .find_map(|i| self.entities.get(&callsign[..i])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let cty = Cty::load("cty.dat");
        assert!(cty.is_ok());
    }

    #[test]
    fn null_lookup() {
        let cty = Cty::load("cty.dat").unwrap();
        let entity = cty.lookup("012");
        assert!(entity.is_none());
    }

    #[test]
    fn prefix_lookup() {
        let cty = Cty::load("cty.dat").unwrap();
        let entity = cty.lookup("DL1ABC").unwrap();
        assert_eq!(entity.name, "Fed. Rep. of Germany");
    }

    #[test]
    fn alias_lookup() {
        let cty = Cty::load("cty.dat").unwrap();
        let entity = cty.lookup("S6ABC").unwrap();
        assert_eq!(entity.name, "Singapore");
    }

    #[test]
    fn exact_lookup() {
        let cty = Cty::load("cty.dat").unwrap();
        let entity = cty.lookup("BS7H").unwrap();
        assert_eq!(entity.name, "Scarborough Reef");
    }



}
