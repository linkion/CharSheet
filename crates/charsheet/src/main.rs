mod bridge {
    pub mod application;
    pub mod library;
    pub mod sheet;
}

use cxx_qt_lib::{
    QGuiApplication, QMap, QMapPair_QString_QVariant, QQmlApplicationEngine, QQuickStyle, QString,
    QUrl, QVariant,
};

const USAGE: &str = "Usage: charsheet [--open <character name or id>] [--tab <0-6>]";

struct Args {
    open: String,
    tab: i32,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        open: String::new(),
        tab: 0,
    };
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--open" => args.open = iter.next().ok_or("--open needs a character name or id")?,
            "--tab" => {
                let tab = iter.next().ok_or("--tab needs a number")?;
                args.tab = tab.parse().map_err(|_| format!("invalid tab: {tab}"))?;
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(args)
}

fn main() {
    let args = parse_args().unwrap_or_else(|e| {
        eprintln!("{e}\n{USAGE}");
        std::process::exit(2);
    });

    let process_args: Vec<String> = std::env::args().collect();
    let mut app = bridge::application::ffi::new_application(&process_args);
    QGuiApplication::set_desktop_file_name(&QString::from("io.github.porave.CharSheet"));
    if std::env::var_os("QT_QUICK_CONTROLS_STYLE").is_none() {
        QQuickStyle::set_style(&QString::from("org.kde.desktop"));
    }

    let mut properties = QMap::<QMapPair_QString_QVariant>::default();
    properties.insert(
        QString::from("startCharacter"),
        QVariant::from(&QString::from(&args.open)),
    );
    properties.insert(QString::from("startTab"), QVariant::from(&args.tab));

    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut engine) = engine.as_mut() {
        engine.as_mut().set_initial_properties(&properties);
        engine.load(&QUrl::from(
            "qrc:/qt/qml/io/github/porave/charsheet/qml/Main.qml",
        ));
    }
    if let Some(app) = app.as_mut() {
        bridge::application::ffi::exec_application(app);
    }
}
