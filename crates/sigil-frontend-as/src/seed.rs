//! seed: the previous pass's tables as one pass reads them, and the record of
//! every answer the pass took from them.
//!
//! A pass is a deterministic function of the source, the options, the pass
//! number and the tables the previous pass produced (its SEED). The loop in
//! `eval::run_passes` may stop after pass N instead of running pass N+1 when
//! pass N+1 would execute exactly as pass N did, and it executes exactly as pass
//! N did when every answer pass N took from its seed is the answer pass N+1
//! would take from pass N's output. Every other answer a pass gets is computed
//! by that pass from those answers, so the two executions agree step by step.
//!
//! That argument is only as good as the record of answers, so the record is
//! kept HERE, where the seed is, and nowhere else: each seeded table lives in a
//! [`Seeded`] whose fields are private to this module. The only methods that
//! hand out anything the seed decides are the reading methods below, and each
//! of them records its answer before returning it. Code outside this module
//! cannot read a seed without being recorded, because it cannot reach one.
//!
//! What is recorded is the answer the caller SAW, for a key this pass had not
//! yet written: a key the pass already wrote answers from the pass's own work,
//! which the argument above covers by induction. A recorded key that the pass
//! writes later still has its first answer recorded, and that answer is the one
//! checked.

use sigil_ir::symbols::{SymbolTable, SymbolValue};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};

/// A table a pass is seeded with: what one read of a key answers, and whether a
/// recorded answer still holds against the table the pass produced.
pub(crate) trait Store: Clone {
    /// Everything a reader can learn about one key.
    type Answer: Clone + PartialEq;
    fn answer(&self, key: &str) -> Self::Answer;
    /// The answer, for a report.
    fn describe(answer: &Self::Answer) -> String;
}

impl Store for SymbolTable {
    /// The value, as [`SymbolTable::resolve`] gives it to every reader in the
    /// front end (a poisoned entry and an absent one both answer `None`).
    type Answer = Option<i64>;
    fn answer(&self, key: &str) -> Option<i64> {
        self.resolve(key, Some(""))
    }
    fn describe(answer: &Option<i64>) -> String {
        format!("{answer:?}")
    }
}

impl Store for HashSet<String> {
    type Answer = bool;
    fn answer(&self, key: &str) -> bool {
        self.contains(key)
    }
    fn describe(answer: &bool) -> String {
        answer.to_string()
    }
}

impl<V: Clone + PartialEq> Store for BTreeMap<String, V> {
    /// The definition itself: a reader that found the key used everything in
    /// it (a macro's parameters and body lines with their spans, a function's
    /// parameters and body tokens).
    type Answer = Option<V>;
    fn answer(&self, key: &str) -> Option<V> {
        self.get(key).cloned()
    }
    fn describe(answer: &Option<V>) -> String {
        if answer.is_some() { "defined" } else { "undefined" }.to_string()
    }
}

/// One seeded table: the seed with this pass's writes on top, the keys this
/// pass wrote, and the recorded answers.
///
/// `table` holds the seed's entry for every key not in `written`, because
/// the writing methods (`define`, `insert`) are the only things that change it,
/// and each records the key.
pub(crate) struct Seeded<S: Store> {
    table: S,
    written: HashSet<String>,
    reads: RefCell<HashMap<String, S::Answer>>,
}

impl<S: Store> Seeded<S> {
    pub(crate) fn new(seed: S) -> Self {
        Seeded { table: seed, written: HashSet::new(), reads: RefCell::new(HashMap::new()) }
    }

    /// THE READ CHOKE POINT. Every method that returns something the seed
    /// decides calls this first.
    fn note(&self, key: &str) {
        if self.written.contains(key) {
            return;
        }
        let mut reads = self.reads.borrow_mut();
        if !reads.contains_key(key) {
            reads.insert(key.to_string(), self.table.answer(key));
        }
    }

    /// Whether this pass has written `key`. Not a read of the seed.
    pub(crate) fn written_here(&self, key: &str) -> bool {
        self.written.contains(key)
    }

    /// The produced table and the record of what was read from the seed.
    pub(crate) fn finish(self) -> (S, ReadLog<S>) {
        (self.table, ReadLog { reads: self.reads.into_inner().into_iter().collect() })
    }
}

impl Seeded<SymbolTable> {
    pub(crate) fn resolve(&self, key: &str) -> Option<i64> {
        self.note(key);
        self.table.answer(key)
    }
    pub(crate) fn define(&mut self, key: &str, value: SymbolValue) {
        self.table.define(key, value);
        self.written.insert(key.to_string());
    }
}

impl Seeded<HashSet<String>> {
    pub(crate) fn contains(&self, key: &str) -> bool {
        self.note(key);
        self.table.contains(key)
    }
    pub(crate) fn insert(&mut self, key: String) {
        self.written.insert(key.clone());
        self.table.insert(key);
    }
}

impl<V: Clone + PartialEq> Seeded<BTreeMap<String, V>> {
    pub(crate) fn get(&self, key: &str) -> Option<&V> {
        self.note(key);
        self.table.get(key)
    }
    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.note(key);
        self.table.contains_key(key)
    }
    pub(crate) fn insert(&mut self, key: String, value: V) {
        self.written.insert(key.clone());
        self.table.insert(key, value);
    }
}

/// The answers one pass took from one seeded table, by key.
pub(crate) struct ReadLog<S: Store> {
    reads: Vec<(String, S::Answer)>,
}

impl<S: Store> ReadLog<S> {
    /// The first recorded read that `out`, the table the pass produced, answers
    /// differently, as `key:seen->now`.
    fn first_change(&self, out: &S) -> Option<String> {
        self.reads.iter().find(|(k, a)| out.answer(k) != *a).map(|(k, a)| {
            format!("{k}:{}->{}", S::describe(a), S::describe(&out.answer(k)))
        })
    }
    pub(crate) fn len(&self) -> usize {
        self.reads.len()
    }
}

/// The symbol environment as a pass reads it: the seeded table, plus the index
/// of which macro-expansion instances filed each name on the previous pass
/// (built from the seed, so reading it is a read of the seed too).
pub(crate) struct SeededEnv {
    env: Seeded<SymbolTable>,
    owners: HashMap<String, Vec<String>>,
    owner_reads: RefCell<HashMap<String, Option<Vec<String>>>>,
}

impl SeededEnv {
    pub(crate) fn new(seed: SymbolTable) -> Self {
        let owners = index_instance_owned(&seed);
        SeededEnv { env: Seeded::new(seed), owners, owner_reads: RefCell::new(HashMap::new()) }
    }

    /// The value a reference to the fully qualified `key` resolves to.
    pub(crate) fn resolve(&self, key: &str) -> Option<i64> {
        self.env.resolve(key)
    }

    /// Bind `key`. The only writer.
    pub(crate) fn define(&mut self, key: &str, value: SymbolValue) {
        self.env.define(key, value);
    }

    /// Whether THIS pass has bound `key` yet. Not a read of the seed.
    ///
    /// This is what `DEFINED(NAME)` answers from, and the environment's value
    /// cannot answer it: the environment is seeded from the previous pass (that
    /// is how a forward reference gets a value), so it holds every symbol the
    /// program will ever define from the first line of every pass after the
    /// first, and asking it "is this defined yet" gets a yes for a label a
    /// hundred lines below.
    ///
    /// asl's own answer is positional and RESETS each pass, which is measured
    /// rather than assumed: in a two-pass file whose `jmp Later` resolves to
    /// `Later`'s address on pass 2, `dc.b DEFINED(Later)` above it is `00` on
    /// pass 2 just as it was on pass 1 (probe `db.asm`, `2 passes`, `0 errors`,
    /// exit 0). Carrying the value forward and carrying the DEFINEDNESS forward
    /// are separate, and asl carries only the first.
    ///
    /// [`Self::define`] is the only writer and records the key, so a form that
    /// binds a name without it cannot exist.
    pub(crate) fn defined_this_pass(&self, key: &str) -> bool {
        self.env.written_here(key)
    }

    /// The instance keys (` exp#N`) the previous pass filed `name` under: the
    /// seed's instance-filed keys indexed by the name they file, built once per
    /// pass. `eval`'s `plain_label_scope` reads it so a forward reference inside
    /// a macro body finds a definition the body scan could not claim.
    pub(crate) fn owners(&self, name: &str) -> Option<&Vec<String>> {
        let got = self.owners.get(name);
        self.owner_reads.borrow_mut().entry(name.to_string()).or_insert_with(|| got.cloned());
        got
    }

    pub(crate) fn finish(self) -> (SymbolTable, EnvReadLog) {
        let (table, values) = self.env.finish();
        (table, EnvReadLog { values, owners: self.owner_reads.into_inner() })
    }
}

/// What one pass read from its seed environment.
pub(crate) struct EnvReadLog {
    values: ReadLog<SymbolTable>,
    owners: HashMap<String, Option<Vec<String>>>,
}

/// Everything one pass read from every table it was seeded with.
pub(crate) struct SeedReads<M: Clone + PartialEq, F: Clone + PartialEq> {
    pub(crate) env: EnvReadLog,
    pub(crate) labels: ReadLog<HashSet<String>>,
    pub(crate) label_ref_equs: ReadLog<HashSet<String>>,
    pub(crate) macros: ReadLog<BTreeMap<String, M>>,
    pub(crate) functions: ReadLog<BTreeMap<String, F>>,
}

impl<M: Clone + PartialEq, F: Clone + PartialEq> SeedReads<M, F> {
    /// The first recorded read the produced tables answer differently, named by
    /// table, or `None` when every one holds and the next pass would execute as
    /// this one did.
    pub(crate) fn first_change(
        &self,
        env: &SymbolTable,
        labels: &HashSet<String>,
        label_ref_equs: &HashSet<String>,
        macros: &BTreeMap<String, M>,
        functions: &BTreeMap<String, F>,
    ) -> Option<String> {
        if let Some(c) = self.env.values.first_change(env) {
            return Some(format!("symbol {c}"));
        }
        if let Some(c) = self.labels.first_change(labels) {
            return Some(format!("label {c}"));
        }
        if let Some(c) = self.label_ref_equs.first_change(label_ref_equs) {
            return Some(format!("label-equ {c}"));
        }
        if let Some(c) = self.macros.first_change(macros) {
            return Some(format!("macro {c}"));
        }
        if let Some(c) = self.functions.first_change(functions) {
            return Some(format!("function {c}"));
        }
        if !self.env.owners.is_empty() {
            let now = index_instance_owned(env);
            if let Some((k, a)) = self.env.owners.iter().find(|(k, a)| now.get(*k) != a.as_ref()) {
                return Some(format!("owner {k}:{a:?}->{:?}", now.get(k)));
            }
        }
        None
    }

    /// How many answers were recorded, per table, for the phase line.
    pub(crate) fn census(&self) -> String {
        format!(
            "reads_env={}\treads_owners={}\treads_labels={}\treads_label_equs={}\treads_macros={}\treads_functions={}",
            self.env.values.len(),
            self.env.owners.len(),
            self.labels.len(),
            self.label_ref_equs.len(),
            self.macros.len(),
            self.functions.len()
        )
    }
}

/// Index an environment's instance-filed keys by the name they file:
/// ` exp#7.Lp` contributes `Lp -> [" exp#7"]`, ` exp#3. nameless+#2` contributes
/// ` nameless+#2 -> [" exp#3"]`. A name several instances filed (a macro invoked
/// twice) maps to every one of them; the reader picks the one that is live.
///
/// Only ` exp#` keys are read: the `.`-local scopes a macro expansion opens
/// (` macro#N`) have their own resolution rule and are not instances.
fn index_instance_owned(env: &SymbolTable) -> HashMap<String, Vec<String>> {
    const PREFIX: &str = " exp#";
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (key, _) in env.iter() {
        let Some(rest) = key.strip_prefix(PREFIX) else { continue };
        let Some(dot) = rest.find('.') else { continue };
        if dot == 0 || !rest[..dot].bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let instance = &key[..PREFIX.len() + dot];
        out.entry(rest[dot + 1..].to_string()).or_default().push(instance.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(pairs: &[(&str, i64)]) -> SymbolTable {
        let mut t = SymbolTable::new();
        for (k, v) in pairs {
            t.define(k, SymbolValue::Int(*v));
        }
        t
    }

    /// A key read before this pass wrote it is recorded with the seed's answer,
    /// absent included; a key read after is not recorded at all.
    #[test]
    fn reads_of_the_seed_are_recorded_and_reads_of_this_pass_are_not() {
        let mut e = SeededEnv::new(table(&[("A", 1)]));
        assert_eq!(e.resolve("A"), Some(1));
        assert_eq!(e.resolve("B"), None);
        e.define("C", SymbolValue::Int(3));
        assert_eq!(e.resolve("C"), Some(3));
        let (out, log) = e.finish();
        let mut got = log.values.reads.clone();
        got.sort();
        assert_eq!(got, vec![("A".to_string(), Some(1)), ("B".to_string(), None)]);
        assert_eq!(out.resolve("C", Some("")), Some(3));
    }

    /// A name that was absent when read and is present in the produced table is
    /// a change: the next pass would see it at that point.
    #[test]
    fn an_added_name_that_was_read_as_absent_is_a_change() {
        let mut e = SeededEnv::new(table(&[]));
        assert_eq!(e.resolve("Later"), None);
        e.define("Later", SymbolValue::Int(5));
        let (out, log) = e.finish();
        assert_eq!(log.values.first_change(&out).as_deref(), Some("Later:None->Some(5)"));
    }

    /// A name added but never read before its definition is not a change.
    #[test]
    fn an_added_name_nobody_read_early_is_not_a_change() {
        let mut e = SeededEnv::new(table(&[]));
        e.define("Later", SymbolValue::Int(5));
        assert_eq!(e.resolve("Later"), Some(5));
        let (out, log) = e.finish();
        assert_eq!(log.values.first_change(&out), None);
    }

    /// A definition read from the seed and then rewritten by the pass holds
    /// only when the rewrite is the same definition.
    #[test]
    fn a_seeded_definition_the_pass_rewrote_holds_only_if_unchanged() {
        let mut seed = BTreeMap::new();
        seed.insert("M".to_string(), 1u8);
        let mut same = Seeded::new(seed.clone());
        assert!(same.contains_key("M"));
        same.insert("M".to_string(), 1u8);
        let (out, log) = same.finish();
        assert_eq!(log.first_change(&out), None);
        let mut other = Seeded::new(seed);
        assert!(other.contains_key("M"));
        other.insert("M".to_string(), 2u8);
        let (out, log) = other.finish();
        assert!(log.first_change(&out).is_some());
    }

    /// A macro name read as absent and defined later is a change; one read
    /// after its definition is not.
    #[test]
    fn map_presence_is_checked_both_ways() {
        let mut m: Seeded<BTreeMap<String, u8>> = Seeded::new(BTreeMap::new());
        assert!(m.get("Early").is_none());
        m.insert("Early".to_string(), 0);
        m.insert("Late".to_string(), 0);
        assert!(m.get("Late").is_some());
        let (out, log) = m.finish();
        assert_eq!(log.first_change(&out).as_deref(), Some("Early:undefined->defined"));
    }

    /// The previous pass's instance index is read through the environment too,
    /// and an answer that the produced environment would change is reported.
    #[test]
    fn an_owner_answer_that_moves_is_a_change() {
        let e = SeededEnv::new(table(&[]));
        assert!(e.owners("Lp").is_none());
        let (_, log) = e.finish();
        let reads: SeedReads<u8, u8> = SeedReads {
            env: log,
            labels: Seeded::new(HashSet::new()).finish().1,
            label_ref_equs: Seeded::new(HashSet::new()).finish().1,
            macros: Seeded::new(BTreeMap::new()).finish().1,
            functions: Seeded::new(BTreeMap::new()).finish().1,
        };
        let now = table(&[(" exp#3.Lp", 7)]);
        let empty = HashSet::new();
        let change = reads.first_change(&now, &empty, &empty, &BTreeMap::new(), &BTreeMap::new());
        assert!(change.is_some_and(|c| c.starts_with("owner Lp")));
    }
}
