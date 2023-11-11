# Rust parser for [Amateur Radio Country Files](https://www.country-files.com/)

Example usage:

```
use cty_rs::Cty;

fn main() {
    let cty = Cty::new("cty.dat").unwrap();
    let country = cty.lookup("9V1AAA").unwrap();
    println!("{} {}", country.name, country.continent);
}
```