package fail.acheron.floatview.speech

import java.util.Locale

/** Engines can expose either ISO 639-1 or ISO 639-2 language identifiers. */
internal fun isEnglishSpeechLocale(locale: Locale): Boolean =
    locale.language == Locale.ENGLISH.language || locale.language == Locale.ENGLISH.isO3Language
