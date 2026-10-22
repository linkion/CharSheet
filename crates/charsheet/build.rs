use cxx_qt_build::{CxxQtBuilder, QResource, QResources, QmlFile, QmlModule};
use std::path::PathBuf;

const FONTS: &[&str] = &[
    "fonts/Bookinsanity-Bold.otf",
    "fonts/Bookinsanity-BoldItalic.otf",
    "fonts/Bookinsanity-Italic.otf",
    "fonts/Bookinsanity-Regular.otf",
    "fonts/NodestoCapsCondensed-Bold.otf",
    "fonts/NodestoCapsCondensed-Regular.otf",
    "fonts/ScalySans-Bold.otf",
    "fonts/ScalySans-BoldItalic.otf",
    "fonts/ScalySans-Italic.otf",
    "fonts/ScalySans-Regular.otf",
    "fonts/ScalySansCaps-Bold.otf",
    "fonts/ScalySansCaps-Regular.otf",
];

const THEME_FILES: &[&str] = &[
    "theme/Parchment.colors",
    "theme/Stone.colors",
    "theme/parchment.png",
    "theme/stone.png",
];

fn main() {
    let builder =
        CxxQtBuilder::new_qml_module(QmlModule::new("io.github.porave.charsheet").qml_files([
            QmlFile::from("qml/App.qml").singleton(true),
            QmlFile::from("qml/Style.qml").singleton(true),
            QmlFile::from("qml/AbilityBox.qml"),
            QmlFile::from("qml/ActionButton.qml"),
            QmlFile::from("qml/ActionListEditor.qml"),
            QmlFile::from("qml/ActionRow.qml"),
            QmlFile::from("qml/BannerRule.qml"),
            QmlFile::from("qml/BuildTab.qml"),
            QmlFile::from("qml/CombatTab.qml"),
            QmlFile::from("qml/DiamondPip.qml"),
            QmlFile::from("qml/DiceBox.qml"),
            QmlFile::from("qml/DiceOverlay.qml"),
            QmlFile::from("qml/Die3D.qml"),
            QmlFile::from("qml/DieAtlas.qml"),
            QmlFile::from("qml/EffectEditor.qml"),
            QmlFile::from("qml/FeaturesTab.qml"),
            QmlFile::from("qml/FrameTitle.qml"),
            QmlFile::from("qml/Flourish.qml"),
            QmlFile::from("qml/GridPaper.qml"),
            QmlFile::from("qml/InventoryTab.qml"),
            QmlFile::from("qml/ItemDialog.qml"),
            QmlFile::from("qml/LibraryPage.qml"),
            QmlFile::from("qml/Main.qml"),
            QmlFile::from("qml/MainTab.qml"),
            QmlFile::from("qml/ModifierListEditor.qml"),
            QmlFile::from("qml/NotesTab.qml"),
            QmlFile::from("qml/OvalBadge.qml"),
            QmlFile::from("qml/PipRow.qml"),
            QmlFile::from("qml/ProficiencyDot.qml"),
            QmlFile::from("qml/Prose.qml"),
            QmlFile::from("qml/ProseLine.qml"),
            QmlFile::from("qml/RollButton.qml"),
            QmlFile::from("qml/RollLogDrawer.qml"),
            QmlFile::from("qml/Section.qml"),
            QmlFile::from("qml/SheetArea.qml"),
            QmlFile::from("qml/SheetBackground.qml"),
            QmlFile::from("qml/SheetCard.qml"),
            QmlFile::from("qml/SheetCheck.qml"),
            QmlFile::from("qml/SheetField.qml"),
            QmlFile::from("qml/SheetFrame.qml"),
            QmlFile::from("qml/SheetPage.qml"),
            QmlFile::from("qml/SheetRow.qml"),
            QmlFile::from("qml/SheetSpin.qml"),
            QmlFile::from("qml/SheetTab.qml"),
            QmlFile::from("qml/ShieldShape.qml"),
            QmlFile::from("qml/SpellsTab.qml"),
            QmlFile::from("qml/StatTile.qml"),
            QmlFile::from("qml/ThemeAction.qml"),
            QmlFile::from("qml/WeaponAttackRow.qml"),
        ]))
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qt_module("Quick3D")
        .qt_module("QuickShapes")
        .qt_module("Widgets")
        .include_dir("cpp")
        .files([
            "src/bridge/application.rs",
            "src/bridge/dice.rs",
            "src/bridge/library.rs",
            "src/bridge/markdown.rs",
            "src/bridge/sheet.rs",
            "src/bridge/theme.rs",
        ])
        // Served from :/fonts and :/theme rather than under the QML module.
        .qrc_resources(
            QResources::new()
                .resource(QResource::new().prefix("/").files(FONTS.iter().chain(THEME_FILES))),
        );

    // KDE Frameworks, for applying the bundled color schemes (cpp/theme.h).
    println!("cargo::rerun-if-env-changed=KF6_INCLUDE_DIR");
    let kf6 = PathBuf::from(std::env::var("KF6_INCLUDE_DIR").unwrap_or("/usr/include/KF6".into()));
    println!("cargo::rustc-link-lib=KF6ColorScheme");
    println!("cargo::rustc-link-lib=KF6ConfigCore");

    // SAFETY: only adds include paths and a warning flag (GCC 16 warns about Qt's own
    // headers otherwise).
    let builder = unsafe {
        builder.cc_builder(|cc| {
            cc.flag_if_supported("-Wno-sfinae-incomplete");
            for module in ["KColorScheme", "KConfigCore", "KConfig"] {
                cc.include(kf6.join(module));
            }
        })
    };
    builder.build();
}
