use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub inheritance: Option<String>,
    #[serde(flatten)]
    pub backend: HashMap<String, BackendConfig>,
    pub command: CmdConfig,
}

#[derive(Debug, Deserialize)]
pub struct BackendConfig {
    #[serde(flatten)]
    pub verbs: HashMap<String, FlagConfig>,
    #[serde(flatten)]
    pub f: FlagConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct FlagConfig {
    pub flag_alias: HashMap<String, String>,
    pub flag: HashMap<String, FlagValue>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CmdConfig {
    #[serde(flatten)]
    pub t: HashMap<String, String>,
    pub script: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FlagValue {
    Bool(bool),
    String(String),
    List(Vec<String>),
}

impl Profile {
    /// ## resolve command flags:
    /// 1. replace flag alias with flag in backend
    /// 2. replace flag alias with flag in backend.verb
    /// 3. fill flag in backend and backend.verb
    /// 4. return resolved flags
    pub fn resolve_flags<T: AsRef<str>, I: IntoIterator<Item = T>>(
        &self,
        args: I,
        backend: &str,
        verb: &str,
    ) -> Vec<String> {
        let mut resolved = self.replace_alias_flags(args, backend, verb);
        self.append_flags(&mut resolved, backend, verb);
        resolved
    }
    fn replace_alias_flags<T, I>(&self, args: I, backend: &str, verb: &str) -> Vec<String>
    where
        T: AsRef<str>,
        I: IntoIterator<Item = T>,
    {
        args.into_iter()
            .map(|flag| {
                self.bk_flag_alias(backend, Some(verb))
                    .and_then(|f| f.get(flag.as_ref()))
                    .or_else(|| {
                        self.bk_flag_alias(backend, None)
                            .and_then(|f| f.get(flag.as_ref()))
                    })
                    .map(|s| s.as_str())
                    .unwrap_or(flag.as_ref())
                    .to_owned()
            })
            .collect()
    }

    fn append_flags(&self, args: &mut Vec<String>, backend: &str, verb: &str) {
        let flag_sources = [
            self.bk_flag(backend, Some(verb)),
            self.bk_flag(backend, None),
        ];

        for flags in flag_sources.into_iter().flatten() {
            for (k, v) in flags {
                if args.contains(&k) {
                    continue;
                }
                self.flag_value(args, k, v);
            }
        }
    }

    fn flag_value(&self, args: &mut Vec<String>, key: &str, value: &FlagValue) {
        match value {
            FlagValue::Bool(true) => args.push(key.to_string()),
            FlagValue::String(s) => {
                args.push(key.to_string());
                args.push(s.to_string());
            }
            FlagValue::List(l) => {
                args.push(key.to_string());
                args.extend(l.iter().cloned());
            }
            _ => {}
        }
    }

    fn bk_flag_alias(&self, backend: &str, verb: Option<&str>) -> Option<&HashMap<String, String>> {
        self.backend
            .get(backend)
            .and_then(|b| {
                verb.and_then(|v| b.verbs.get(v))
                    .and_then(|v| Some(&v.flag_alias))
            })
            .or_else(|| self.backend.get(backend).map(|b| &b.f.flag_alias))
    }

    fn bk_flag(&self, backend: &str, verb: Option<&str>) -> Option<&HashMap<String, FlagValue>> {
        self.backend
            .get(backend)
            .and_then(|b| verb.and_then(|v| b.verbs.get(v)))
            .map(|b| &b.flag)
            .or_else(|| self.backend.get(backend).map(|b| &b.f.flag))
    }
}
