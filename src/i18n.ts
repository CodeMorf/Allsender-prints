import es from "./locales/es.json";
import esDO from "./locales/es-DO.json";
import it from "./locales/it.json";
import en from "./locales/en.json";
import fr from "./locales/fr.json";

export type Locale = "es" | "es-DO" | "it" | "en" | "fr";
export type TranslationKey = keyof typeof es;

const dictionaries: Record<Locale, Record<TranslationKey, string>> = {
  es,
  "es-DO": esDO,
  it,
  en,
  fr
};

export const localeOptions: Array<{ value: Locale; label: string }> = [
  { value: "es", label: "Español" },
  { value: "es-DO", label: "Español (República Dominicana)" },
  { value: "it", label: "Italiano" },
  { value: "en", label: "English" },
  { value: "fr", label: "Français" }
];
// Labels above must stay UTF-8 (á, é, ú, ç, etc.).

export function normalizeLocale(value?: string): Locale {
  const raw = (value || "").trim().toLowerCase();
  if (raw === "do" || raw === "es-do" || raw === "es_do") return "es-DO";
  if (raw.startsWith("it")) return "it";
  if (raw.startsWith("en")) return "en";
  if (raw.startsWith("fr")) return "fr";
  return "es";
}

export function translator(locale: Locale) {
  return (key: TranslationKey): string => dictionaries[locale][key] ?? dictionaries.es[key] ?? key;
}
