use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    let builder =
        CxxQtBuilder::new_qml_module(QmlModule::new("io.github.porave.charsheet").qml_files([
            QmlFile::from("qml/App.qml").singleton(true),
            QmlFile::from("qml/ActionButton.qml"),
            QmlFile::from("qml/ActionListEditor.qml"),
            QmlFile::from("qml/ActionRow.qml"),
            QmlFile::from("qml/BuildTab.qml"),
            QmlFile::from("qml/CombatTab.qml"),
            QmlFile::from("qml/DiceBox.qml"),
            QmlFile::from("qml/EffectEditor.qml"),
            QmlFile::from("qml/FeaturesTab.qml"),
            QmlFile::from("qml/InventoryTab.qml"),
            QmlFile::from("qml/ItemDialog.qml"),
            QmlFile::from("qml/LibraryPage.qml"),
            QmlFile::from("qml/Main.qml"),
            QmlFile::from("qml/MainTab.qml"),
            QmlFile::from("qml/ModifierListEditor.qml"),
            QmlFile::from("qml/NotesTab.qml"),
            QmlFile::from("qml/PipRow.qml"),
            QmlFile::from("qml/RollButton.qml"),
            QmlFile::from("qml/RollLogDrawer.qml"),
            QmlFile::from("qml/Section.qml"),
            QmlFile::from("qml/SheetArea.qml"),
            QmlFile::from("qml/SheetCheck.qml"),
            QmlFile::from("qml/SheetField.qml"),
            QmlFile::from("qml/SheetPage.qml"),
            QmlFile::from("qml/SheetSpin.qml"),
            QmlFile::from("qml/SheetTab.qml"),
            QmlFile::from("qml/SpellsTab.qml"),
            QmlFile::from("qml/StatTile.qml"),
            QmlFile::from("qml/WeaponAttackRow.qml"),
        ]))
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qt_module("Widgets")
        .include_dir("cpp")
        .files([
            "src/bridge/application.rs",
            "src/bridge/library.rs",
            "src/bridge/sheet.rs",
        ]);

    // SAFETY: only adds a warning flag; GCC 16 warns about Qt's own headers otherwise.
    let builder = unsafe {
        builder.cc_builder(|cc| {
            cc.flag_if_supported("-Wno-sfinae-incomplete");
        })
    };
    builder.build();
}
