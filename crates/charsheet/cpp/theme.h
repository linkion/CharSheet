#pragma once

// The app's look: "stone" (the 2024 character sheet), "parchment" (the Player's
// Handbook) or "system" (the desktop's own colors and fonts).
//
// The themed modes swap in a bundled KDE color scheme. KColorScheme reads qApp's
// KDE_COLOR_SCHEME_PATH property, so setting it together with the application palette
// recolors every control and every Kirigami.Theme color, as KColorSchemeManager does.

#include <KColorScheme>
#include <KSharedConfig>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QSettings>
#include <QtCore/QStandardPaths>
#include <QtCore/QVariant>
#include <QtGui/QFont>
#include <QtGui/QFontDatabase>
#include <QtWidgets/QApplication>

#include "rust/cxx.h"

inline void register_fonts()
{
    const char *files[] = {
        "ScalySans-Regular.otf",
        "ScalySans-Bold.otf",
        "ScalySans-Italic.otf",
        "ScalySans-BoldItalic.otf",
        "ScalySansCaps-Regular.otf",
        "ScalySansCaps-Bold.otf",
        "Bookinsanity-Regular.otf",
        "Bookinsanity-Bold.otf",
        "Bookinsanity-Italic.otf",
        "Bookinsanity-BoldItalic.otf",
        "NodestoCapsCondensed-Regular.otf",
        "NodestoCapsCondensed-Bold.otf",
    };
    for (const char *file : files) {
        QFontDatabase::addApplicationFont(QStringLiteral(":/fonts/") + QString::fromLatin1(file));
    }
}

// KConfig reads real files only, so the bundled scheme is copied out of the resources.
inline QString extract_scheme(const QString &name)
{
    const QString dir = QStandardPaths::writableLocation(QStandardPaths::CacheLocation);
    QDir().mkpath(dir);
    const QString path = dir + QLatin1Char('/') + name + QStringLiteral(".colors");
    QFile::remove(path);
    QFile::copy(QStringLiteral(":/theme/") + name + QStringLiteral(".colors"), path);
    QFile::setPermissions(path, QFile::ReadOwner | QFile::WriteOwner);
    return path;
}

inline void apply_theme(rust::Str mode)
{
    const QString m = QString::fromUtf8(mode.data(), static_cast<qsizetype>(mode.size()));
    // Anything unrecognised gets the default, Stone.
    if (m != QLatin1String("system")) {
        const QString path = extract_scheme(m == QLatin1String("parchment") ? QStringLiteral("Parchment") : QStringLiteral("Stone"));
        qApp->setProperty("KDE_COLOR_SCHEME_PATH", path);
        qApp->setPalette(KColorScheme::createApplicationPalette(KSharedConfig::openConfig(path)));
        QFont font(QStringLiteral("Scaly Sans Remake"));
        font.setPointSizeF(QFontDatabase::systemFont(QFontDatabase::GeneralFont).pointSizeF() * 1.1);
        QApplication::setFont(font);
    } else {
        qApp->setProperty("KDE_COLOR_SCHEME_PATH", QVariant());
        qApp->setPalette(KColorScheme::createApplicationPalette(KSharedConfig::openConfig()));
        QApplication::setFont(QFontDatabase::systemFont(QFontDatabase::GeneralFont));
    }
}

inline rust::String system_font_family()
{
    return rust::String(QFontDatabase::systemFont(QFontDatabase::GeneralFont).family().toStdString());
}

inline rust::String saved_theme()
{
    const QString mode = QSettings().value(QStringLiteral("appearance/theme"), QStringLiteral("stone")).toString();
    return rust::String(mode.toStdString());
}

inline void save_theme(rust::Str mode)
{
    QSettings().setValue(QStringLiteral("appearance/theme"),
                         QString::fromUtf8(mode.data(), static_cast<qsizetype>(mode.size())));
}
