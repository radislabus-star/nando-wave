pub(crate) fn parse_wave_tick_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(u64, u8), String> {
    let input = args
        .next()
        .ok_or_else(|| String::from("missing input byte"))?;
    let input_byte = parse_u8(&input, "input byte")?;

    let seed = match args.next() {
        Some(value) => parse_u64(&value, "seed")?,
        None => 1,
    };

    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }

    Ok((seed, input_byte))
}

pub(crate) fn parse_snapshot_save_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(u64, u8, String), String> {
    let input = args
        .next()
        .ok_or_else(|| String::from("missing input byte"))?;
    let input_byte = parse_u8(&input, "input byte")?;

    let seed = match args.next() {
        Some(value) => parse_u64(&value, "seed")?,
        None => 1,
    };

    let path = args
        .next()
        .unwrap_or_else(|| format!("target/snapshots/stage2-seed{seed}-byte{input_byte}.nws1"));

    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }

    Ok((seed, input_byte, path))
}

pub(crate) fn parse_bench_stage2_tick_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(u64, usize), String> {
    let seed = match args.next() {
        Some(value) => parse_u64(&value, "seed")?,
        None => 1,
    };
    let ticks = match args.next() {
        Some(value) => parse_usize(&value, "ticks")?,
        None => 10_000,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if ticks == 0 {
        return Err(String::from("ticks must be greater than zero"));
    }
    Ok((seed, ticks))
}

pub(crate) fn parse_live_byte_train_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(u64, String), String> {
    let first = args
        .next()
        .ok_or_else(|| String::from("missing training text"))?;

    let (seed, mut parts) = match first.parse::<u64>() {
        Ok(seed) => (seed, Vec::new()),
        Err(_) => (1, vec![first]),
    };
    parts.extend(args);

    if parts.is_empty() {
        return Err(String::from("missing training text"));
    }

    Ok((seed, parts.join(" ")))
}

pub(crate) fn parse_optional_seed_arg(
    mut args: impl Iterator<Item = String>,
) -> Result<u64, String> {
    let seed = match args.next() {
        Some(value) => parse_u64(&value, "seed")?,
        None => 1,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    Ok(seed)
}

pub(crate) fn parse_live_grok_trace_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(u64, usize, usize), String> {
    let seed = match args.next() {
        Some(value) => parse_u64(&value, "seed")?,
        None => 1,
    };
    let epochs = match args.next() {
        Some(value) => parse_usize(&value, "epochs")?,
        None => 64,
    };
    let interval = match args.next() {
        Some(value) => parse_usize(&value, "interval")?,
        None => 8,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if epochs == 0 {
        return Err(String::from("epochs must be greater than zero"));
    }
    if interval == 0 {
        return Err(String::from("interval must be greater than zero"));
    }
    Ok((seed, epochs, interval))
}

pub(crate) fn parse_live_grok_sweep_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(usize, usize), String> {
    let epochs = match args.next() {
        Some(value) => parse_usize(&value, "epochs")?,
        None => 64,
    };
    let interval = match args.next() {
        Some(value) => parse_usize(&value, "interval")?,
        None => 8,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if epochs == 0 {
        return Err(String::from("epochs must be greater than zero"));
    }
    if interval == 0 {
        return Err(String::from("interval must be greater than zero"));
    }
    Ok((epochs, interval))
}

pub(crate) fn parse_u8(value: &str, label: &str) -> Result<u8, String> {
    value
        .parse::<u8>()
        .map_err(|error| format!("invalid {label} '{value}': {error}"))
}

pub(crate) fn parse_u64(value: &str, label: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {label} '{value}': {error}"))
}

pub(crate) fn parse_periodic_args(
    mut args: impl Iterator<Item = String>,
) -> Result<nando_eval::PeriodicTaskConfig, String> {
    let mut config = nando_eval::PeriodicTaskConfig::default();

    if let Some(value) = args.next() {
        config.seed = parse_u64(&value, "seed")?;
    }
    if let Some(value) = args.next() {
        config.cases = parse_usize(&value, "cases")?;
    }
    if let Some(value) = args.next() {
        config.start = parse_u8(&value, "start")?;
    }
    if let Some(value) = args.next() {
        config.step = parse_u8(&value, "step")?;
    }
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if config.cases == 0 {
        return Err(String::from("cases must be greater than zero"));
    }

    Ok(config)
}

pub(crate) fn parse_phase_composition_args(
    mut args: impl Iterator<Item = String>,
) -> Result<nando_eval::PhaseCompositionConfig, String> {
    let mut config = nando_eval::PhaseCompositionConfig::default();

    if let Some(value) = args.next() {
        config.seed = parse_u64(&value, "seed")?;
    }
    if let Some(value) = args.next() {
        config.cases = parse_usize(&value, "cases")?;
    }
    if let Some(value) = args.next() {
        config.start = parse_u8(&value, "start")?;
    }
    if let Some(value) = args.next() {
        config.input_step = parse_u8(&value, "input-step")?;
    }
    if let Some(value) = args.next() {
        config.phase_step = parse_u8(&value, "phase-step")?;
    }
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if config.cases == 0 {
        return Err(String::from("cases must be greater than zero"));
    }

    Ok(config)
}

pub(crate) fn parse_phase_holdout_args(
    mut args: impl Iterator<Item = String>,
) -> Result<
    (
        nando_eval::PhaseCompositionConfig,
        nando_eval::PhaseCompositionConfig,
    ),
    String,
> {
    let mut train = nando_eval::PhaseCompositionConfig::default();
    let mut holdout = nando_eval::PhaseCompositionConfig {
        seed: 97,
        cases: train.cases,
        start: 31,
        input_step: 29,
        phase_step: 7,
    };

    if let Some(value) = args.next() {
        train.seed = parse_u64(&value, "train-seed")?;
    }
    if let Some(value) = args.next() {
        holdout.seed = parse_u64(&value, "holdout-seed")?;
    }
    if let Some(value) = args.next() {
        let cases = parse_usize(&value, "cases")?;
        train.cases = cases;
        holdout.cases = cases;
    }
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if train.cases == 0 {
        return Err(String::from("cases must be greater than zero"));
    }

    Ok((train, holdout))
}

pub(crate) fn parse_cases_only_args(
    mut args: impl Iterator<Item = String>,
) -> Result<usize, String> {
    let cases = match args.next() {
        Some(value) => parse_usize(&value, "cases")?,
        None => nando_eval::PhaseCompositionConfig::default().cases,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if cases == 0 {
        return Err(String::from("cases must be greater than zero"));
    }
    Ok(cases)
}

pub(crate) fn parse_seed_pair_cases_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(u64, u64, usize), String> {
    let train_seed = match args.next() {
        Some(value) => parse_u64(&value, "train seed")?,
        None => 13,
    };
    let holdout_seed = match args.next() {
        Some(value) => parse_u64(&value, "holdout seed")?,
        None => 97,
    };
    let cases = match args.next() {
        Some(value) => parse_usize(&value, "cases")?,
        None => nando_eval::PhaseCompositionConfig::default().cases,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if cases == 0 {
        return Err(String::from("cases must be greater than zero"));
    }

    Ok((train_seed, holdout_seed, cases))
}

pub(crate) fn parse_usize(value: &str, label: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("invalid {label} '{value}': {error}"))
}
