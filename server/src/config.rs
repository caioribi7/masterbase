//! Comando `masterbase`: arquivo de configuração, perguntas da primeira vez e serviço do sistema.
//! Variáveis de ambiente sempre valem mais que o arquivo.

use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::Command;

pub fn help() {
    println!(
        "Masterbase {}\n\n\
         masterbase              abre o painel\n\
         masterbase configurar   muda banco, porta e quem pode acessar\n\
         masterbase servico      deixa rodando sozinho ao ligar o servidor (Linux, com sudo)\n\
         masterbase versao       mostra a versão\n\n\
         Configuração em {}",
        env!("CARGO_PKG_VERSION"),
        path().display()
    );
}

pub fn path() -> PathBuf {
    if let Ok(p) = std::env::var("MB_CONFIG") {
        return p.into();
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join(".config"));
    base.join("masterbase").join("masterbase.env")
}

fn home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| ".".into())
}

fn read() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(path()).unwrap_or_default();
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

/// Carrega o arquivo nas variáveis de ambiente que ainda não existem. Roda antes do runtime (uma thread só).
pub fn load() {
    for (k, v) in read() {
        if std::env::var_os(&k).is_none() {
            std::env::set_var(k, v);
        }
    }
}

fn ask(question: &str, default: &str) -> String {
    if default.is_empty() {
        print!("{question}: ");
    } else {
        print!("{question} [{default}]: ");
    }
    std::io::stdout().flush().ok();
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
        std::process::exit(1);
    }
    let line = line.trim();
    if line.is_empty() { default.to_string() } else { line.to_string() }
}

pub fn configure() {
    if !std::io::stdin().is_terminal() {
        eprintln!("rode `masterbase configurar` num terminal");
        std::process::exit(1);
    }
    let mut cfg = read();
    println!("Configurar o Masterbase. Enter mantém o valor entre colchetes.\n");

    let mut url = ask("Endereço do banco (postgresql://usuário:senha@servidor:5432/banco)", cfg.get("DATABASE_URL").map_or("", |s| s.as_str()));
    while !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
        url = ask("Precisa começar com postgresql://", "");
    }
    cfg.insert("DATABASE_URL".into(), url);

    let (_, old_port) = cfg.get("MB_ADDR").and_then(|a| a.rsplit_once(':')).unwrap_or(("", "8080"));
    let port = ask("Porta do painel", old_port);
    let port = port.parse::<u16>().map(|p| p.to_string()).unwrap_or_else(|_| "8080".into());

    println!("\nQuem vai abrir o painel?");
    println!("  1) Só este computador");
    println!("  2) Outras máquinas da rede local (sem HTTPS)");
    println!("  3) Pela internet, com HTTPS na frente (Caddy ou Nginx)");
    let old = match (cfg.get("MB_ADDR").map(|a| a.starts_with("0.0.0.0")), cfg.contains_key("MB_INSECURE_COOKIE")) {
        (Some(true), _) => "2",
        _ => "1",
    };
    let who = ask("Escolha", old);
    cfg.remove("MB_INSECURE_COOKIE");
    match who.as_str() {
        "2" => {
            cfg.insert("MB_ADDR".into(), format!("0.0.0.0:{port}"));
            cfg.insert("MB_INSECURE_COOKIE".into(), "1".into());
            println!("\nAtenção: sem HTTPS, a senha passa aberta pela rede. Use só em rede de confiança.");
        }
        "3" => {
            cfg.insert("MB_ADDR".into(), format!("127.0.0.1:{port}"));
            println!("\nAponte o Caddy ou Nginx para 127.0.0.1:{port}. Exemplo em docs/instalacao.md.");
        }
        _ => {
            cfg.insert("MB_ADDR".into(), format!("127.0.0.1:{port}"));
        }
    }

    cfg.entry("MB_BACKUP_DIR".into()).or_insert_with(|| {
        let base = std::env::var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|_| home().join(".local/share"));
        base.join("masterbase/backups").to_string_lossy().into_owned()
    });

    save(&cfg);
    println!("\nSalvo em {}", path().display());
}

fn save(cfg: &BTreeMap<String, String>) {
    let file = path();
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).expect("não deu para criar a pasta de configuração");
    }
    let mut text = String::from("# Configuração do Masterbase. Edite com `masterbase configurar`.\n");
    for (k, v) in cfg {
        text.push_str(&format!("{k}={v}\n"));
    }
    std::fs::write(&file, text).expect("não deu para salvar a configuração");
    // A senha do banco está aqui: só o dono lê.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).ok();
    }
}

/// Instala o serviço do systemd rodando como a pessoa que chamou `sudo masterbase servico`.
pub fn service() {
    if Command::new("systemctl").arg("--version").output().is_err() {
        eprintln!("Este sistema não tem systemd. Rode `masterbase` com o gerenciador de serviços que você usa.");
        std::process::exit(1);
    }
    let root = Command::new("id").arg("-u").output().map(|o| o.stdout.starts_with(b"0")).unwrap_or(false);
    if !root {
        eprintln!("Rode com sudo: sudo masterbase servico");
        std::process::exit(1);
    }
    let user = std::env::var("SUDO_USER").unwrap_or_else(|_| "root".into());
    let home = Command::new("getent").args(["passwd", &user]).output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|l| l.trim().split(':').nth(5).map(String::from))
        .unwrap_or_else(|| "/root".into());
    let config = std::env::var("MB_CONFIG").unwrap_or_else(|_| format!("{home}/.config/masterbase/masterbase.env"));
    if !std::path::Path::new(&config).exists() {
        eprintln!("Nenhuma configuração em {config}. Rode `masterbase configurar` antes (sem sudo).");
        std::process::exit(1);
    }
    let exe = std::env::current_exe().expect("caminho do masterbase");
    let unit = format!(
        "[Unit]\nDescription=Masterbase\nAfter=network.target postgresql.service\n\n\
         [Service]\nUser={user}\nEnvironment=MB_CONFIG={config}\nExecStart={}\nRestart=on-failure\nNoNewPrivileges=true\n\n\
         [Install]\nWantedBy=multi-user.target\n",
        exe.display()
    );
    std::fs::write("/etc/systemd/system/masterbase.service", unit).expect("não deu para gravar o serviço");
    let ok = Command::new("systemctl").arg("daemon-reload").status().map(|s| s.success()).unwrap_or(false)
        && Command::new("systemctl").args(["enable", "--now", "masterbase"]).status().map(|s| s.success()).unwrap_or(false);
    if ok {
        println!("Pronto. O Masterbase liga sozinho junto com o servidor.");
        println!("Ver se está rodando: systemctl status masterbase");
    } else {
        eprintln!("O serviço não subiu. Veja: journalctl -u masterbase");
        std::process::exit(1);
    }
}

/// Abre o navegador quando dá: só em terminal, só em endereço local.
pub fn open_browser(url: &str) {
    if !std::io::stdout().is_terminal() {
        return;
    }
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(url);
        c
    } else if std::env::var_os("WSL_DISTRO_NAME").is_some() {
        let mut c = Command::new("cmd.exe");
        c.args(["/c", "start", url]);
        c
    } else if std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some() {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    } else {
        return;
    };
    let _ = cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
}
