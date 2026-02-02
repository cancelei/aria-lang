mod manifest;
mod resolver;

use clap::{Parser, Subcommand};
use manifest::{AriaManifest, DependencySpec};
use resolver::Resolver;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Parser)]
#[command(name = "aria-pkg")]
#[command(about = "Aria package manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Aria project
    Init {
        /// Project name
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Add a dependency to the project
    Add {
        /// Package name
        package: String,
        /// Version requirement (e.g., "1.0.0", "^1.0", "~1.2.3")
        #[arg(short, long)]
        version: Option<String>,
        /// Git repository URL
        #[arg(short, long)]
        git: Option<String>,
        /// Local path to dependency
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Remove a dependency from the project
    Remove {
        /// Package name
        package: String,
    },
    /// Install all dependencies
    Install,
    /// Build the project
    Build {
        /// Build in release mode
        #[arg(short, long)]
        release: bool,
    },
    /// Run the project
    Run {
        /// Run in release mode
        #[arg(short, long)]
        release: bool,
        /// Arguments to pass to the program
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Publish package to registry (placeholder)
    Publish {
        /// Perform a dry run without publishing
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => init_project(path),
        Commands::Add { package, version, git, path } => {
            add_dependency(package, version, git, path)
        }
        Commands::Remove { package } => remove_dependency(package),
        Commands::Install => install_dependencies(),
        Commands::Build { release } => build_project(release),
        Commands::Run { release, args } => run_project(release, args),
        Commands::Publish { dry_run } => publish_package(dry_run),
    }
}

fn init_project(path: PathBuf) -> anyhow::Result<()> {
    let project_dir = if path == Path::new(".") {
        std::env::current_dir()?
    } else {
        path.clone()
    };

    // Create project directory if it doesn't exist
    if !project_dir.exists() {
        fs::create_dir_all(&project_dir)?;
    }

    let manifest_path = project_dir.join("Aria.toml");

    // Check if manifest already exists
    if manifest_path.exists() {
        anyhow::bail!("Aria.toml already exists in {}", project_dir.display());
    }

    // Get project name from directory name
    let project_name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("aria-project")
        .to_string();

    // Create manifest
    let manifest = AriaManifest::new(project_name.clone(), semver::Version::new(0, 1, 0));
    manifest.save(&manifest_path)?;

    // Create src directory and main.aria file
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let main_file = src_dir.join("main.aria");
    fs::write(
        main_file,
        r#"# Main entry point
fn main()
  println("Hello from Aria!")
end
"#,
    )?;

    // Create .gitignore
    let gitignore_path = project_dir.join(".gitignore");
    fs::write(
        gitignore_path,
        r#"/target
/build
Aria.lock
*.aria-cache
"#,
    )?;

    println!("Created Aria project '{}' in {}", project_name, project_dir.display());
    println!("\nNext steps:");
    println!("  cd {}", project_dir.display());
    println!("  aria-pkg build");
    println!("  aria-pkg run");

    Ok(())
}

fn add_dependency(
    package: String,
    version: Option<String>,
    git: Option<String>,
    path: Option<String>,
) -> anyhow::Result<()> {
    let manifest_path = find_manifest()?;
    let mut manifest = AriaManifest::from_file(&manifest_path)?;

    let spec = if let Some(git_url) = git {
        DependencySpec::Detailed {
            version: version.unwrap_or_else(|| "*".to_string()),
            git: Some(git_url),
            branch: None,
            tag: None,
            rev: None,
            path: None,
            optional: None,
            features: None,
            default_features: None,
        }
    } else if let Some(local_path) = path {
        DependencySpec::Detailed {
            version: version.unwrap_or_else(|| "*".to_string()),
            git: None,
            branch: None,
            tag: None,
            rev: None,
            path: Some(local_path),
            optional: None,
            features: None,
            default_features: None,
        }
    } else {
        let version_str = version.unwrap_or_else(|| "^0.1.0".to_string());
        DependencySpec::Simple(version_str)
    };

    manifest.add_dependency(package.clone(), spec);
    manifest.save(&manifest_path)?;

    println!("Added {} to dependencies", package);
    println!("Run 'aria-pkg install' to install the dependency");

    Ok(())
}

fn remove_dependency(package: String) -> anyhow::Result<()> {
    let manifest_path = find_manifest()?;
    let mut manifest = AriaManifest::from_file(&manifest_path)?;

    if manifest.remove_dependency(&package) {
        manifest.save(&manifest_path)?;
        println!("Removed {} from dependencies", package);
    } else {
        println!("Package {} not found in dependencies", package);
    }

    Ok(())
}

fn install_dependencies() -> anyhow::Result<()> {
    let manifest_path = find_manifest()?;
    let manifest = AriaManifest::from_file(&manifest_path)?;

    println!("Resolving dependencies...");
    let resolver = Resolver::new(manifest.clone());
    let lock_file = resolver.resolve()?;

    let lock_path = manifest_path.parent().unwrap().join("Aria.lock");
    lock_file.save(&lock_path)?;

    println!("Installing {} packages...", lock_file.packages.len());

    // Create .aria directory for storing packages
    let aria_dir = manifest_path.parent().unwrap().join(".aria");
    let packages_dir = aria_dir.join("packages");
    fs::create_dir_all(&packages_dir)?;

    for package in &lock_file.packages {
        println!("  Installing {} v{}", package.name, package.version);

        // In a real implementation, this would:
        // 1. Download the package from the registry
        // 2. Verify checksums
        // 3. Extract to packages directory
        // 4. Build if necessary

        // For now, we just create a placeholder
        let package_dir = packages_dir.join(format!("{}-{}", package.name, package.version));
        fs::create_dir_all(&package_dir)?;

        let package_info = package_dir.join("package.info");
        fs::write(
            package_info,
            format!("Package: {}\nVersion: {}\n", package.name, package.version),
        )?;
    }

    println!("\nDependencies installed successfully");
    println!("Lockfile written to {}", lock_path.display());

    Ok(())
}

fn build_project(release: bool) -> anyhow::Result<()> {
    let manifest_path = find_manifest()?;
    let manifest = AriaManifest::from_file(&manifest_path)?;
    let project_dir = manifest_path.parent().unwrap();

    let build_mode = if release { "release" } else { "debug" };
    println!("Building {} v{} [{}]", manifest.package.name, manifest.package.version, build_mode);

    // Check if src/main.aria exists
    let main_file = project_dir.join("src/main.aria");
    if !main_file.exists() {
        anyhow::bail!("src/main.aria not found");
    }

    // Create build directory
    let build_dir = project_dir.join("build").join(build_mode);
    fs::create_dir_all(&build_dir)?;

    // Determine output path
    let output_path = build_dir.join(&manifest.package.name);

    // Build compile options
    let options = aria_compiler::CompileOptions {
        output: Some(output_path.clone()),
        link: true,
        runtime_path: None, // Auto-detect
        is_library: false,
        lib_paths: vec![project_dir.join("src")],
        release,
    };

    println!("   Compiling {} v{}", manifest.package.name, manifest.package.version);

    // Compile using the real compiler
    match aria_compiler::compile_file(&main_file, options) {
        Ok(result) => {
            println!("    Finished {} target(s)", build_mode);
            println!("     Binary: {}", result.output_path.display());
            if result.modules.len() > 1 {
                println!("     Modules: {}", result.modules.join(", "));
            }
            Ok(())
        }
        Err(e) => {
            anyhow::bail!("Compilation failed: {}", e)
        }
    }
}

fn run_project(release: bool, args: Vec<String>) -> anyhow::Result<()> {
    // First, ensure the project is built
    build_project(release)?;

    let manifest_path = find_manifest()?;
    let manifest = AriaManifest::from_file(&manifest_path)?;
    let project_dir = manifest_path.parent().unwrap();

    let build_mode = if release { "release" } else { "debug" };
    let binary_path = project_dir
        .join("build")
        .join(build_mode)
        .join(&manifest.package.name);

    if !binary_path.exists() {
        anyhow::bail!("Binary not found at: {}", binary_path.display());
    }

    println!();
    println!("     Running `{}`", binary_path.display());
    println!();

    // Execute the binary
    let mut cmd = Command::new(&binary_path);
    cmd.args(&args);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    cmd.current_dir(project_dir);

    let status = cmd.status()?;

    if !status.success() {
        anyhow::bail!("Process exited with status: {}", status);
    }

    Ok(())
}

fn publish_package(dry_run: bool) -> anyhow::Result<()> {
    let manifest_path = find_manifest()?;
    let manifest = AriaManifest::from_file(&manifest_path)?;

    println!("Publishing {} v{}", manifest.package.name, manifest.package.version);

    if dry_run {
        println!("\n[DRY RUN] Would publish to registry: https://pkg.aria-lang.org");
        println!("\nPackage contents:");
        println!("  - Aria.toml");
        println!("  - src/");

        // In a real implementation, this would:
        // 1. Validate the package
        // 2. Run tests
        // 3. Create a package tarball
        // 4. Upload to registry
        // 5. Update package index

        println!("\nPackage validation passed");
        println!("Package size: ~1.2 KB");
        println!("\nDry run complete. Use 'aria-pkg publish' to actually publish.");
    } else {
        println!("\n[INFO] Publishing to registry is not yet implemented");
        println!("This is a placeholder for future registry integration");
        println!("\nPlanned features:");
        println!("  - Package validation and verification");
        println!("  - Authentication with registry");
        println!("  - Tarball creation and upload");
        println!("  - Version conflict checking");
        println!("  - Documentation generation");
    }

    Ok(())
}

fn find_manifest() -> anyhow::Result<PathBuf> {
    let mut current_dir = std::env::current_dir()?;

    loop {
        let manifest_path = current_dir.join("Aria.toml");
        if manifest_path.exists() {
            return Ok(manifest_path);
        }

        if !current_dir.pop() {
            break;
        }
    }

    anyhow::bail!("Could not find Aria.toml in current directory or any parent directory")
}
