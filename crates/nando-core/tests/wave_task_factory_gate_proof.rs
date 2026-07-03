use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

const EXACT_LOOKUP_MAX_MILLI: u16 = 20;
const L2_NEIGHBOR_MAX_MILLI: u16 = 200;
const BAYESIAN_PAIRWISE_MAX_MILLI: u16 = 700;
const MARKOV_BIGRAM_MAX_MILLI: u16 = 700;
const TARGET_LEAK_MAX_MILLI: u16 = 200;
const SINGLE_TOKEN_MAX_MILLI: u16 = 250;
const NEAR_NEGATIVE_MIN_MILLI: u16 = 120;

#[derive(Clone, Debug)]
struct PredictorTask {
    input_text: String,
    target_text: String,
    near_negative_text: String,
    source_group: String,
    task_kind: &'static str,
}

#[derive(Clone, Debug)]
struct TaskGateMetrics {
    tasks_total: usize,
    heldout_tasks: usize,
    source_groups: usize,
    heldout_source_groups: usize,
    task_kinds: usize,
    exact_lookup_accuracy_milli: u16,
    l2_neighbor_accuracy_milli: u16,
    bayesian_pairwise_accuracy_milli: u16,
    markov_bigram_accuracy_milli: u16,
    target_leak_milli: u16,
    near_negative_similarity_milli: u16,
    single_token_ratio_milli: u16,
    task_factory_verdict: &'static str,
}

impl TaskGateMetrics {
    fn valid_operator_pressure(&self) -> bool {
        self.task_factory_verdict == "VALID_OPERATOR_PRESSURE_PROVEN"
    }
}

#[test]
fn task_factory_rejects_dictionary_word_sequence_as_semantic_grokking_source() {
    let words = corpus_lines("russian_words_300k.txt");
    let tasks = dictionary_sequence_tasks(&words[..2_000]);
    let metrics = evaluate_task_quality(&tasks);
    eprintln!("dictionary task gate metrics: {metrics:#?}");

    assert!(metrics.source_groups >= 8, "metrics={metrics:#?}");
    assert!(metrics.heldout_source_groups >= 1, "metrics={metrics:#?}");
    assert!(!metrics.valid_operator_pressure(), "metrics={metrics:#?}");
    assert!(
        metrics.single_token_ratio_milli >= 950,
        "dictionary tasks must be visible as single-token sequence artifacts: {metrics:#?}"
    );
    assert_eq!(metrics.task_factory_verdict, "REJECT_SINGLE_TOKEN_SEQUENCE");
}

#[test]
fn task_factory_accepts_structured_next_line_tasks_and_breaks_v2_shortcuts() {
    let lines = corpus_lines("organ128_train_v1.txt");
    let tasks = next_line_tasks(&lines);
    let metrics = evaluate_task_quality(&tasks);
    eprintln!("structured next-line task gate metrics: {metrics:#?}");

    assert!(metrics.tasks_total >= 40, "metrics={metrics:#?}");
    assert!(metrics.heldout_tasks >= 8, "metrics={metrics:#?}");
    assert!(metrics.source_groups >= 4, "metrics={metrics:#?}");
    assert!(metrics.heldout_source_groups >= 1, "metrics={metrics:#?}");
    assert_v2_shortcuts_broken(&metrics);
    assert!(metrics.valid_operator_pressure(), "metrics={metrics:#?}");
}

#[test]
fn task_factory_rejects_local_multi_kind_source_when_l2_neighbor_solves_it() {
    let train_lines = corpus_lines("organ128_train_v1.txt");
    let dialog_lines = corpus_lines("organ128_dialog_ru_en_v1.tsv");
    let mut tasks = Vec::new();
    tasks.extend(next_line_tasks(&train_lines));
    tasks.extend(procedure_window_tasks(&train_lines));
    tasks.extend(dialogue_reply_tasks(&dialog_lines));

    let metrics = evaluate_task_quality(&tasks);
    eprintln!("local multi-kind task gate metrics: {metrics:#?}");

    assert!(metrics.tasks_total >= 180, "metrics={metrics:#?}");
    assert!(metrics.heldout_tasks >= 24, "metrics={metrics:#?}");
    assert!(metrics.source_groups >= 8, "metrics={metrics:#?}");
    assert!(metrics.heldout_source_groups >= 2, "metrics={metrics:#?}");
    assert!(metrics.task_kinds >= 3, "metrics={metrics:#?}");
    assert!(
        metrics.l2_neighbor_accuracy_milli > L2_NEIGHBOR_MAX_MILLI,
        "local multi-kind source should expose L2 shortcut weakness: {metrics:#?}"
    );
    assert!(!metrics.valid_operator_pressure(), "metrics={metrics:#?}");
    assert_eq!(metrics.task_factory_verdict, "REJECT_L2_NEIGHBOR_SHORTCUT");
}

fn assert_v2_shortcuts_broken(metrics: &TaskGateMetrics) {
    assert!(
        metrics.exact_lookup_accuracy_milli <= EXACT_LOOKUP_MAX_MILLI,
        "exact lookup shortcut survived: {metrics:#?}"
    );
    assert!(
        metrics.l2_neighbor_accuracy_milli <= L2_NEIGHBOR_MAX_MILLI,
        "L2-neighbor shortcut survived: {metrics:#?}"
    );
    assert!(
        metrics.bayesian_pairwise_accuracy_milli <= BAYESIAN_PAIRWISE_MAX_MILLI,
        "Bayesian conditional-frequency shortcut survived: {metrics:#?}"
    );
    assert!(
        metrics.markov_bigram_accuracy_milli <= MARKOV_BIGRAM_MAX_MILLI,
        "Markov/bigram shortcut survived: {metrics:#?}"
    );
    assert!(
        metrics.target_leak_milli <= TARGET_LEAK_MAX_MILLI,
        "target leaks through input: {metrics:#?}"
    );
    assert!(
        metrics.near_negative_similarity_milli >= NEAR_NEGATIVE_MIN_MILLI,
        "near-negative is too alien to stress the same field: {metrics:#?}"
    );
    assert!(
        metrics.single_token_ratio_milli <= SINGLE_TOKEN_MAX_MILLI,
        "structured tasks collapsed into wordlist behavior: {metrics:#?}"
    );
}

fn next_line_tasks(lines: &[String]) -> Vec<PredictorTask> {
    let candidates = meaningful_lines(lines);
    let mut tasks = Vec::new();

    for index in 0..candidates.len().saturating_sub(1) {
        let input = candidates[index].clone();
        let target = candidates[index + 1].clone();
        let near_negative = nearest_negative_for_target(&target, &candidates, index + 1);
        if input != target && !input.contains(&target) && !target.contains(&input) {
            tasks.push(PredictorTask {
                input_text: input,
                target_text: target,
                near_negative_text: near_negative,
                source_group: format!("organ128_next_{:03}", index / 20),
                task_kind: "next_line",
            });
        }
    }

    tasks
}

fn procedure_window_tasks(lines: &[String]) -> Vec<PredictorTask> {
    let candidates = meaningful_lines(lines);
    let mut tasks = Vec::new();

    for index in 1..candidates.len().saturating_sub(1) {
        let input = format!("{} {}", candidates[index - 1], candidates[index]);
        let target = candidates[index + 1].clone();
        let near_negative = nearest_negative_for_target(&target, &candidates, index + 1);
        if !input.contains(&target) && !target.contains(&input) {
            tasks.push(PredictorTask {
                input_text: input,
                target_text: target,
                near_negative_text: near_negative,
                source_group: format!("organ128_procedure_{:03}", index / 18),
                task_kind: "procedure_window",
            });
        }
    }

    tasks
}

fn dialogue_reply_tasks(lines: &[String]) -> Vec<PredictorTask> {
    let rows = lines
        .iter()
        .skip(1)
        .enumerate()
        .filter_map(|(index, line)| {
            let (prompt, answer) = line.split_once('\t')?;
            if token_count(prompt) < 2 || token_count(answer) < 3 {
                return None;
            }
            let language = if contains_cyrillic(prompt) {
                "ru"
            } else {
                "en"
            };
            Some((
                index,
                prompt.to_string(),
                answer.to_string(),
                format!("organ128_dialog_{language}_{:03}", index / 8),
            ))
        })
        .collect::<Vec<_>>();
    let targets = rows
        .iter()
        .map(|(_, _, answer, _)| answer.clone())
        .collect::<Vec<_>>();

    rows.into_iter()
        .map(|(index, prompt, answer, source_group)| PredictorTask {
            input_text: prompt,
            target_text: answer,
            near_negative_text: nearest_negative_for_target(&targets[index], &targets, index),
            source_group,
            task_kind: "dialogue_reply",
        })
        .collect()
}

fn dictionary_sequence_tasks(words: &[String]) -> Vec<PredictorTask> {
    let mut tasks = Vec::new();
    for index in 0..words.len().saturating_sub(2) {
        tasks.push(PredictorTask {
            input_text: words[index].clone(),
            target_text: words[index + 1].clone(),
            near_negative_text: words[index + 2].clone(),
            source_group: format!("dictionary_chunk_{:03}", index / 200),
            task_kind: "dictionary_sequence",
        });
    }
    tasks
}

fn meaningful_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| token_count(line) >= 4)
        .cloned()
        .collect()
}

fn evaluate_task_quality(tasks: &[PredictorTask]) -> TaskGateMetrics {
    let split = SourceGroupSplit::new(tasks);
    let train = tasks
        .iter()
        .filter(|task| !split.heldout_groups.contains(&task.source_group))
        .collect::<Vec<_>>();
    let heldout = tasks
        .iter()
        .filter(|task| split.heldout_groups.contains(&task.source_group))
        .collect::<Vec<_>>();
    let exact_lookup_accuracy_milli = exact_lookup_accuracy_milli(&train, &heldout);
    let l2_neighbor_accuracy_milli = l2_neighbor_accuracy_milli(&train, &heldout);
    let bayesian_pairwise_accuracy_milli = bayesian_pairwise_accuracy_milli(&train, &heldout);
    let markov_bigram_accuracy_milli = markov_bigram_accuracy_milli(&train, &heldout);
    let target_leak_milli = target_leak_milli(tasks);
    let near_negative_similarity_milli = near_negative_similarity_milli(tasks);
    let single_token_ratio_milli = single_token_ratio_milli(tasks);
    let task_kinds = task_kind_count(tasks);
    let source_groups = split.source_groups;
    let heldout_source_groups = split.heldout_groups.len();
    let task_factory_verdict = task_factory_verdict(TaskGateMetrics {
        tasks_total: tasks.len(),
        heldout_tasks: heldout.len(),
        source_groups,
        heldout_source_groups,
        task_kinds,
        exact_lookup_accuracy_milli,
        l2_neighbor_accuracy_milli,
        bayesian_pairwise_accuracy_milli,
        markov_bigram_accuracy_milli,
        target_leak_milli,
        near_negative_similarity_milli,
        single_token_ratio_milli,
        task_factory_verdict: "PENDING",
    });

    TaskGateMetrics {
        tasks_total: tasks.len(),
        heldout_tasks: heldout.len(),
        source_groups,
        heldout_source_groups,
        task_kinds,
        exact_lookup_accuracy_milli,
        l2_neighbor_accuracy_milli,
        bayesian_pairwise_accuracy_milli,
        markov_bigram_accuracy_milli,
        target_leak_milli,
        near_negative_similarity_milli,
        single_token_ratio_milli,
        task_factory_verdict,
    }
}

struct SourceGroupSplit {
    source_groups: usize,
    heldout_groups: HashSet<String>,
}

impl SourceGroupSplit {
    fn new(tasks: &[PredictorTask]) -> Self {
        let groups = tasks
            .iter()
            .map(|task| task.source_group.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut heldout_groups = groups
            .iter()
            .enumerate()
            .filter(|(index, _)| index % 5 == 0)
            .map(|(_, group)| group.clone())
            .collect::<HashSet<_>>();

        if heldout_groups.is_empty()
            && let Some(group) = groups.last()
        {
            heldout_groups.insert(group.clone());
        }

        Self {
            source_groups: groups.len(),
            heldout_groups,
        }
    }
}

fn task_factory_verdict(metrics: TaskGateMetrics) -> &'static str {
    if metrics.single_token_ratio_milli > SINGLE_TOKEN_MAX_MILLI {
        return "REJECT_SINGLE_TOKEN_SEQUENCE";
    }
    if metrics.heldout_source_groups == 0 || metrics.heldout_tasks == 0 {
        return "REJECT_NO_SOURCE_GROUP_HELDOUT";
    }
    if metrics.exact_lookup_accuracy_milli > EXACT_LOOKUP_MAX_MILLI {
        return "REJECT_EXACT_LOOKUP_SHORTCUT";
    }
    if metrics.l2_neighbor_accuracy_milli > L2_NEIGHBOR_MAX_MILLI {
        return "REJECT_L2_NEIGHBOR_SHORTCUT";
    }
    if metrics.bayesian_pairwise_accuracy_milli > BAYESIAN_PAIRWISE_MAX_MILLI {
        return "REJECT_BAYESIAN_SHORTCUT";
    }
    if metrics.markov_bigram_accuracy_milli > MARKOV_BIGRAM_MAX_MILLI {
        return "REJECT_MARKOV_BIGRAM_SHORTCUT";
    }
    if metrics.target_leak_milli > TARGET_LEAK_MAX_MILLI {
        return "REJECT_TARGET_LEAKAGE";
    }
    if metrics.near_negative_similarity_milli < NEAR_NEGATIVE_MIN_MILLI {
        return "REJECT_NEGATIVE_TOO_ALIEN";
    }
    "VALID_OPERATOR_PRESSURE_PROVEN"
}

fn exact_lookup_accuracy_milli(train: &[&PredictorTask], heldout: &[&PredictorTask]) -> u16 {
    let train_map = train
        .iter()
        .map(|task| (task.input_text.as_str(), task.target_text.as_str()))
        .collect::<HashMap<_, _>>();
    let correct = heldout
        .iter()
        .filter(|task| {
            train_map
                .get(task.input_text.as_str())
                .is_some_and(|target| *target == task.target_text)
        })
        .count();
    milli_ratio(correct, heldout.len())
}

fn l2_neighbor_accuracy_milli(train: &[&PredictorTask], heldout: &[&PredictorTask]) -> u16 {
    let correct = heldout
        .iter()
        .filter(|heldout_task| {
            let nearest = train.iter().max_by(|left, right| {
                let left_score = trigram_jaccard(&heldout_task.input_text, &left.input_text);
                let right_score = trigram_jaccard(&heldout_task.input_text, &right.input_text);
                left_score.total_cmp(&right_score)
            });
            nearest.is_some_and(|task| {
                task.target_text == heldout_task.target_text
                    || trigram_jaccard(&task.target_text, &heldout_task.target_text) >= 0.92
            })
        })
        .count();
    milli_ratio(correct, heldout.len())
}

fn bayesian_pairwise_accuracy_milli(train: &[&PredictorTask], heldout: &[&PredictorTask]) -> u16 {
    let model = BayesianCooccurrenceBaseline::train(train);
    let correct = heldout
        .iter()
        .filter(|task| model.prefers_target_over_negative(task))
        .count();
    milli_ratio(correct, heldout.len())
}

fn markov_bigram_accuracy_milli(train: &[&PredictorTask], heldout: &[&PredictorTask]) -> u16 {
    let model = MarkovBigramBaseline::train(train);
    let correct = heldout
        .iter()
        .filter(|task| model.prefers_target_over_negative(task))
        .count();
    milli_ratio(correct, heldout.len())
}

struct BayesianCooccurrenceBaseline {
    input_counts: HashMap<String, u32>,
    pair_counts: HashMap<(String, String), u32>,
    target_atom_counts: HashMap<String, u32>,
    total_target_atoms: u32,
}

impl BayesianCooccurrenceBaseline {
    fn train(tasks: &[&PredictorTask]) -> Self {
        let mut input_counts = HashMap::new();
        let mut pair_counts = HashMap::new();
        let mut target_atom_counts = HashMap::new();
        let mut total_target_atoms = 0_u32;

        for task in tasks {
            let input_atoms = atom_set(&task.input_text);
            let target_atoms = atom_set(&task.target_text);
            for input_atom in &input_atoms {
                *input_counts.entry(input_atom.clone()).or_default() += 1;
                for target_atom in &target_atoms {
                    *pair_counts
                        .entry((input_atom.clone(), target_atom.clone()))
                        .or_default() += 1;
                }
            }
            for target_atom in target_atoms {
                *target_atom_counts.entry(target_atom).or_default() += 1;
                total_target_atoms += 1;
            }
        }

        Self {
            input_counts,
            pair_counts,
            target_atom_counts,
            total_target_atoms,
        }
    }

    fn prefers_target_over_negative(&self, task: &PredictorTask) -> bool {
        let target_score = self.score(&task.input_text, &task.target_text);
        let negative_score = self.score(&task.input_text, &task.near_negative_text);
        target_score > negative_score
    }

    fn score(&self, input: &str, candidate: &str) -> f32 {
        let input_atoms = atom_set(input);
        let candidate_atoms = atom_set(candidate);
        if input_atoms.is_empty() || candidate_atoms.is_empty() {
            return 0.0;
        }

        let mut score = 0.0_f32;
        for input_atom in &input_atoms {
            let input_count = *self.input_counts.get(input_atom).unwrap_or(&0) as f32;
            for target_atom in &candidate_atoms {
                let pair_count = *self
                    .pair_counts
                    .get(&(input_atom.clone(), target_atom.clone()))
                    .unwrap_or(&0) as f32;
                let target_prior = *self.target_atom_counts.get(target_atom).unwrap_or(&0) as f32;
                let numerator = pair_count + 0.25 * target_prior + 1.0;
                let denominator = input_count + self.total_target_atoms.max(1) as f32;
                score += numerator / denominator;
            }
        }

        score / (input_atoms.len() * candidate_atoms.len()) as f32
    }
}

struct MarkovBigramBaseline {
    transition_counts: HashMap<(String, String), u32>,
    bigram_counts: HashMap<(String, String), u32>,
    unigram_counts: HashMap<String, u32>,
}

impl MarkovBigramBaseline {
    fn train(tasks: &[&PredictorTask]) -> Self {
        let mut transition_counts = HashMap::new();
        let mut bigram_counts = HashMap::new();
        let mut unigram_counts = HashMap::new();

        for task in tasks {
            let input_tokens = tokens(&task.input_text);
            let target_tokens = tokens(&task.target_text);
            if let (Some(input_last), Some(target_first)) =
                (input_tokens.last(), target_tokens.first())
            {
                *transition_counts
                    .entry((input_last.clone(), target_first.clone()))
                    .or_default() += 1;
            }
            for token in &target_tokens {
                *unigram_counts.entry(token.clone()).or_default() += 1;
            }
            for pair in target_tokens.windows(2) {
                *bigram_counts
                    .entry((pair[0].clone(), pair[1].clone()))
                    .or_default() += 1;
            }
        }

        Self {
            transition_counts,
            bigram_counts,
            unigram_counts,
        }
    }

    fn prefers_target_over_negative(&self, task: &PredictorTask) -> bool {
        let target_score = self.score(&task.input_text, &task.target_text);
        let negative_score = self.score(&task.input_text, &task.near_negative_text);
        target_score > negative_score
    }

    fn score(&self, input: &str, candidate: &str) -> f32 {
        let input_tokens = tokens(input);
        let candidate_tokens = tokens(candidate);
        if candidate_tokens.is_empty() {
            return f32::NEG_INFINITY;
        }

        let mut score = 0.0_f32;
        if let (Some(input_last), Some(candidate_first)) =
            (input_tokens.last(), candidate_tokens.first())
        {
            let count = *self
                .transition_counts
                .get(&(input_last.clone(), candidate_first.clone()))
                .unwrap_or(&0);
            score += 4.0 * (count as f32 + 1.0).ln();
        }

        for token in &candidate_tokens {
            let count = *self.unigram_counts.get(token).unwrap_or(&0);
            score += 0.25 * (count as f32 + 1.0).ln();
        }

        for pair in candidate_tokens.windows(2) {
            let count = *self
                .bigram_counts
                .get(&(pair[0].clone(), pair[1].clone()))
                .unwrap_or(&0);
            score += (count as f32 + 1.0).ln();
        }

        score / candidate_tokens.len() as f32
    }
}

fn target_leak_milli(tasks: &[PredictorTask]) -> u16 {
    let leaks = tasks
        .iter()
        .filter(|task| {
            task.input_text.contains(&task.target_text)
                || trigram_jaccard(&task.input_text, &task.target_text) >= 0.85
        })
        .count();
    milli_ratio(leaks, tasks.len())
}

fn near_negative_similarity_milli(tasks: &[PredictorTask]) -> u16 {
    if tasks.is_empty() {
        return 0;
    }
    let sum = tasks
        .iter()
        .map(|task| {
            (trigram_jaccard(&task.target_text, &task.near_negative_text) * 1_000.0).round()
                as usize
        })
        .sum::<usize>();
    (sum / tasks.len()) as u16
}

fn single_token_ratio_milli(tasks: &[PredictorTask]) -> u16 {
    let single_token = tasks
        .iter()
        .filter(|task| {
            token_count(&task.input_text) <= 1
                && token_count(&task.target_text) <= 1
                && task.task_kind == "dictionary_sequence"
        })
        .count();
    milli_ratio(single_token, tasks.len())
}

fn task_kind_count(tasks: &[PredictorTask]) -> usize {
    tasks
        .iter()
        .map(|task| task.task_kind)
        .collect::<HashSet<_>>()
        .len()
}

fn nearest_negative_for_target(target: &str, candidates: &[String], target_index: usize) -> String {
    candidates
        .iter()
        .enumerate()
        .filter(|(index, candidate)| *index != target_index && candidate.as_str() != target)
        .max_by(|(_, left), (_, right)| {
            let left_score = trigram_jaccard(target, left);
            let right_score = trigram_jaccard(target, right);
            left_score.total_cmp(&right_score)
        })
        .map(|(_, candidate)| candidate.clone())
        .unwrap_or_else(|| target.chars().rev().collect())
}

fn trigram_jaccard(left: &str, right: &str) -> f32 {
    let left = trigrams(left);
    let right = trigrams(right);
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }
    let intersection = left.intersection(&right).count();
    let union = left.union(&right).count().max(1);
    intersection as f32 / union as f32
}

fn trigrams(text: &str) -> HashSet<String> {
    let chars = normalize_text(text).chars().collect::<Vec<_>>();
    if chars.len() < 3 {
        return [chars.iter().collect::<String>()].into_iter().collect();
    }
    chars
        .windows(3)
        .map(|window| window.iter().collect::<String>())
        .collect()
}

fn atom_set(text: &str) -> BTreeSet<String> {
    let mut atoms = trigrams(text).into_iter().collect::<BTreeSet<_>>();
    for token in normalize_text(text).split_whitespace() {
        if token.len() >= 3 {
            atoms.insert(format!("tok:{token}"));
        }
    }
    atoms
}

fn tokens(text: &str) -> Vec<String> {
    normalize_text(text)
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|character| character.is_alphanumeric() || character.is_whitespace())
        .collect::<String>()
}

fn token_count(text: &str) -> usize {
    text.split_whitespace().count()
}

fn contains_cyrillic(text: &str) -> bool {
    text.chars()
        .any(|character| ('а'..='я').contains(&character) || ('А'..='Я').contains(&character))
}

fn milli_ratio(numerator: usize, denominator: usize) -> u16 {
    numerator
        .checked_mul(1_000)
        .and_then(|scaled| scaled.checked_div(denominator))
        .unwrap_or(0) as u16
}

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/corpus")
        .join(name)
}

fn corpus_lines(name: &str) -> Vec<String> {
    std::fs::read_to_string(corpus_path(name))
        .unwrap_or_else(|error| panic!("{name} corpus file must be readable: {error}"))
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}
