#pragma once

// QApplication (rather than QGuiApplication) lets the platform theme provide native
// widget-based dialogs, such as KDE's file picker, to QtQuick.Dialogs.

#include <QtWidgets/QApplication>
#include <memory>
#include <string>
#include <vector>

#include "rust/cxx.h"

inline std::unique_ptr<QApplication> new_application(rust::Slice<const rust::String> args)
{
    // QApplication keeps references to argc and argv for its whole lifetime.
    static std::vector<std::string> storage;
    static std::vector<char *> argv;
    static int argc = 0;
    for (const auto &arg : args) {
        storage.emplace_back(arg.data(), arg.size());
    }
    for (auto &arg : storage) {
        argv.push_back(arg.data());
    }
    argv.push_back(nullptr);
    argc = static_cast<int>(storage.size());
    return std::make_unique<QApplication>(argc, argv.data());
}

inline int exec_application(QApplication &)
{
    return QApplication::exec();
}
