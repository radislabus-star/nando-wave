use crate::args::{parse_organ128_modadd_args, parse_organ128_modadd_seed_sweep_args};

pub(crate) fn run_organ128_modadd_eval(args: impl Iterator<Item = String>) -> Result<(), String> {
    let config = parse_organ128_modadd_args(args)?;
    print!("{}", nando_eval::organ128_modadd_eval(config).to_text());
    Ok(())
}

pub(crate) fn run_organ128_modadd_seed_sweep(
    args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let (modulus, train_cases, holdout_cases) = parse_organ128_modadd_seed_sweep_args(args)?;
    print!(
        "{}",
        nando_eval::organ128_modadd_seed_sweep_eval(modulus, train_cases, holdout_cases).to_text()
    );
    Ok(())
}
