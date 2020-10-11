use std::collections::HashMap;

use crate::common::*;
use crate::publishers::{fetch_owners_of_crates, PublisherData, PublisherKind};

pub fn crates(mut args: std::env::ArgsOs) {
    while let Some(arg) = args.next() {
        match arg.to_str() {
            None => bail_bad_arg(arg),
            Some("--") => break, // we pass args after this to cargo-metadata
            _ => bail_unknown_subcommand_arg("crates", arg),
        }
    }

    let dependencies = sourced_dependencies(args);
    complain_about_non_crates_io_crates(&dependencies);
    let (publisher_users, publisher_teams) = fetch_owners_of_crates(&dependencies);

    // Merge maps back together. Ewww. Maybe there's a better way to go about this.
    let mut owners: HashMap<String, Vec<PublisherData>> = HashMap::new();
    for map in &[publisher_users, publisher_teams] {
        for (crate_name, publishers) in map.iter() {
            let entry = owners.entry(crate_name.clone()).or_default();
            entry.extend_from_slice(publishers);
        }
    }

    let mut ordered_owners: Vec<_> = owners.into_iter().collect();
    // Put crates owned by teams first
    ordered_owners.sort_unstable_by_key(|(name, publishers)| {
        (
            publishers
                .iter()
                .filter(|p| p.kind == PublisherKind::team)
                .next()
                .is_none(), // contains at least one team
            usize::MAX - publishers.len(),
            name.clone(),
        )
    });
    for (_name, publishers) in ordered_owners.iter_mut() {
        // For each crate put teams first
        publishers.sort_unstable_by_key(|p| (p.kind, p.login.clone()));
    }

    println!("\nDependency crates with the people and teams that can publish them to crates.io:\n");
    for (i, (crate_name, publishers)) in ordered_owners.iter().enumerate() {
        let pretty_publishers: Vec<String> = publishers
            .iter()
            .map(|p| match p.kind {
                PublisherKind::team => format!("team \"{}\"", p.login),
                PublisherKind::user => format!("{}", p.login),
            })
            .collect();
        let publishers_list = comma_separated_list(&pretty_publishers);
        println!("{}. {}: {}", i + 1, crate_name, publishers_list);
    }

    if ordered_owners.len() > 0 {
        println!("\nNote: there may be outstanding publisher invitations. crates.io provides no way to list them.");
        println!("Invitations are also impossible to revoke, and they never expire.");
        println!("See https://github.com/rust-lang/crates.io/issues/2868 for more info.");
    }
}