import { useEffect, useMemo, useState } from "react";
import { Activity, CheckCircle2, CirclePause, Languages, MonitorUp, Play, Printer, RefreshCw, Save, Settings2, ShieldCheck, Square, Wrench } from "lucide-react";
import logo from "./assets/restaapp-logo.png";
import { localeOptions, normalizeLocale, translator, type Locale } from "./i18n";
import { desktop } from "./lib/desktop";
import type { AppConfig, ConnectionResult, LocalPrinter, RuntimeStatus, ServerPrinter } from "./types";

const defaultConfig: AppConfig = {
  schema_version: 1,
  domain_url: "https://restapp.allsender.tech",
  branch_key: "",
  domain_locked: true,
  locale: "es",
  autostart: true,
  open_drawer_after_print: false,
  poll_ms: 4000,
  idle_poll_ms: 10000,
  printer_mappings: {},
  backup_mappings: {},
  copies: 1,
  app_version: "3.1.1",
  device_id: ""
};

const defaultStatus: RuntimeStatus = {
  running: false,
  state: "paused",
  printed_today: 0
};

function App() {
  const [splash, setSplash] = useState(true);
  const [config, setConfig] = useState<AppConfig>(defaultConfig);
  const [connection, setConnection] = useState<ConnectionResult | null>(null);
  const [localPrinters, setLocalPrinters] = useState<LocalPrinter[]>([]);
  const [serverPrinters, setServerPrinters] = useState<ServerPrinter[]>([]);
  const [status, setStatus] = useState<RuntimeStatus>(defaultStatus);
  const [busy, setBusy] = useState<string | null>(null);
  const [notice, setNotice] = useState<string>("");
  const [view, setView] = useState<"setup" | "dashboard">("setup");
  const locale = normalizeLocale(config.locale);
  const t = useMemo(() => translator(locale), [locale]);

  useEffect(() => {
    const timer = window.setTimeout(() => setSplash(false), 2600);
    desktop.loadConfig().then((saved) => {
      const normalized = { ...defaultConfig, ...saved, locale: normalizeLocale(saved.locale) };
      setConfig(normalized);
      if (normalized.branch_key) setView("dashboard");
    }).catch(() => undefined);
    desktop.status().then(setStatus).catch(() => undefined);
    return () => window.clearTimeout(timer);
  }, []);

  useEffect(() => {
    if (!splash && config.branch_key) {
      refreshAll(false);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [splash]);

  useEffect(() => {
    if (!status.running) return;
    const timer = window.setInterval(() => desktop.status().then(setStatus).catch(() => undefined), 3000);
    return () => window.clearInterval(timer);
  }, [status.running]);

  async function testConnection() {
    if (!config.branch_key.trim()) {
      setNotice(t("connectionIssue"));
      return;
    }
    setBusy("connection");
    setNotice(t("connecting"));
    try {
      const result = await desktop.testConnection(config.branch_key.trim());
      setConnection(result);
      setNotice(result.ok ? t("connected") : t("connectionIssue"));
      if (result.ok) {
        const next = {
          ...config,
          branch_key: config.branch_key.trim(),
          poll_ms: result.recommended_poll_ms || config.poll_ms,
          idle_poll_ms: result.recommended_idle_poll_ms || config.idle_poll_ms,
          locale: result.locale ? normalizeLocale(result.locale) : config.locale,
          // Identidad por sucursal (cada usuario/branch con su key)
          restaurant_name: result.restaurant_name || config.restaurant_name,
          branch_name: result.branch_name || config.branch_name
        };
        setConfig(next);
        setConnection(result);
        await desktop.saveConfig(next);
        await refreshAll(false, next.branch_key);
        setView("dashboard");
      }
    } catch {
      setConnection({ ok: false, message: "" });
      setNotice(t("connectionIssue"));
    } finally {
      setBusy(null);
    }
  }

  async function refreshAll(showNotice = true, key = config.branch_key) {
    setBusy("refresh");
    try {
      const [locals, remotes] = await Promise.all([
        desktop.localPrinters(),
        key ? desktop.serverPrinters(key) : Promise.resolve([])
      ]);
      setLocalPrinters(locals);
      setServerPrinters(remotes);
      if (showNotice) setNotice(`${t("localDevices")}: ${locals.length}`);
    } catch {
      if (showNotice) setNotice(t("noPrinters"));
    } finally {
      setBusy(null);
    }
  }

  async function save() {
    setBusy("save");
    try {
      await desktop.saveConfig(config);
      await desktop.setAutostart(config.autostart);
      setNotice(t("saved"));
    } catch {
      setNotice(t("connectionIssue"));
    } finally {
      setBusy(null);
    }
  }

  async function toggleService() {
    setBusy("service");
    try {
      await save();
      const next = status.running ? await desktop.stop() : await desktop.start();
      setStatus(next);
      setNotice(next.running ? t("serviceStarted") : t("serviceStopped"));
    } catch {
      setNotice(t("chooseMapping"));
    } finally {
      setBusy(null);
    }
  }

  async function printTest(printerName: string) {
    if (!printerName) return;
    setBusy(`test:${printerName}`);
    try {
      await desktop.printTest(printerName, locale);
      setNotice(t("testSent"));
    } catch {
      setNotice(t("testFailed"));
    } finally {
      setBusy(null);
    }
  }

  const statusLabel = status.state === "ready" || status.state === "printing" ? t("ready")
    : status.state === "reconnecting" ? t("reconnecting")
    : status.state === "setup_required" ? t("setupRequired") : t("paused");

  if (splash) {
    return (
      <main className="splash">
        <div className="splash-glow" />
        <img className="splash-logo" src={logo} alt="RestaAPP" />
        <h1>{t("appName")}</h1>
        <p>{t("by")}</p>
        <div className="splash-progress"><span /></div>
        <small>{t("preparing")}</small>
      </main>
    );
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <img src={logo} alt="RestaAPP" />
          <div><strong>{t("appName")}</strong><span>{t("by")}</span></div>
        </div>
        <nav>
          <button className={view === "dashboard" ? "active" : ""} onClick={() => setView("dashboard")}><Activity size={18}/>{t("dashboard")}</button>
          <button className={view === "setup" ? "active" : ""} onClick={() => setView("setup")}><Settings2 size={18}/>{t("openSettings")}</button>
        </nav>
        <div className="sidebar-footer">
          <button onClick={() => desktop.openLogs().catch(() => undefined)}><Wrench size={17}/>{t("diagnostics")}</button>
          <button onClick={async () => {
            try {
              const path = await desktop.exportDiagnostics();
              setNotice(`${t("diagExported")}`);
              console.info("diag", path);
            } catch {
              setNotice(t("connectionIssue"));
            }
          }}><Wrench size={17}/>{t("exportDiag")}</button>
          <span>{t("version")} {config.app_version}</span>
          <span className="device-id">{t("device")}: {config.device_id ? config.device_id.slice(0, 8) : "—"}…</span>
        </div>
      </aside>

      <section className="content">
        <header className="topbar">
          <div>
            <h1>{view === "setup" ? t("connection") : t("dashboard")}</h1>
            <p>{view === "setup" ? t("welcome") : (
              [config.restaurant_name || connection?.restaurant_name, config.branch_name || connection?.branch_name]
                .filter(Boolean)
                .join(" · ") || t("connectedBranch")
            )}</p>
          </div>
          <div className={`status-pill ${status.running ? "online" : "paused"}`}>
            {status.running ? <CheckCircle2 size={17}/> : <CirclePause size={17}/>} {statusLabel}
          </div>
        </header>

        {notice && <div className="notice"><ShieldCheck size={18}/><span>{notice}</span><button onClick={() => setNotice("")}>×</button></div>}

        {view === "setup" ? (
          <div className="setup-grid">
            <section className="card setup-card">
              <div className="card-title"><MonitorUp size={20}/><div><h2>{t("connection")}</h2><p>{t("domainLocked")}</p></div></div>
              <label>{t("branchKey")}
                <input type="password" autoComplete="off" value={config.branch_key} onChange={(e) => setConfig({ ...config, branch_key: e.target.value })} placeholder="••••••••••••••••••••" />
                <small>{t("branchKeyHint")}</small>
              </label>
              <label>{t("language")}
                <select value={locale} onChange={(e) => setConfig({ ...config, locale: normalizeLocale(e.target.value) })}>
                  {localeOptions.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
                </select>
              </label>
              <div className="toggle-row">
                <div><strong>{t("startWindows")}</strong><span>RestaAPP Printer</span></div>
                <button className={`switch ${config.autostart ? "on" : ""}`} onClick={() => setConfig({ ...config, autostart: !config.autostart })}><span /></button>
              </div>
              <button className="primary full" disabled={busy === "connection"} onClick={testConnection}>
                {busy === "connection" ? <RefreshCw className="spin" size={18}/> : <ShieldCheck size={18}/>} {t("testConnection")}
              </button>
            </section>

            <section className="card visual-card">
              <img src={logo} alt="RestaAPP" />
              <h2>RestaAPP Printer</h2>
              <p>Windows 10/11 · x64</p>
              <div className="feature"><CheckCircle2 size={17}/><span>Segundo plano y bandeja de Windows</span></div>
              <div className="feature"><CheckCircle2 size={17}/><span>Impresión por área: Cocina, Bar y Caja</span></div>
              <div className="feature"><CheckCircle2 size={17}/><span>Reconexión y prevención de duplicados</span></div>
            </section>
          </div>
        ) : (
          <>
            <div className="metrics">
              <section className="metric"><span>{t("status")}</span><strong>{statusLabel}</strong><Activity size={22}/></section>
              <section className="metric"><span>{t("today")}</span><strong>{status.printed_today}</strong><Printer size={22}/></section>
              <section className="metric"><span>{t("lastPrint")}</span><strong>{status.last_print_at ? new Date(status.last_print_at).toLocaleTimeString(locale) : "—"}</strong><CheckCircle2 size={22}/></section>
            </div>

            <section className="card printers-card">
              <div className="card-header">
                <div className="card-title"><Printer size={20}/><div><h2>{t("printers")}</h2><p>{t("localDevices")}: {localPrinters.length}</p></div></div>
                <button className="secondary" onClick={() => refreshAll()} disabled={busy === "refresh"}><RefreshCw className={busy === "refresh" ? "spin" : ""} size={17}/>{t("refresh")}</button>
              </div>

              {serverPrinters.length === 0 ? <div className="empty"><Printer size={30}/><strong>{t("noServerPrinters")}</strong></div> : (
                <div className="printer-list">
                  {serverPrinters.map((server) => {
                    const mapped = config.printer_mappings[String(server.id)] || "";
                    return (
                      <div className="printer-row" key={server.id}>
                        <div className="server-printer"><span>{t("serverArea")}</span><strong>{server.name}</strong><small>{server.print_format || "thermal80mm"}</small></div>
                        <div className="mapping-arrow">→</div>
                        <label className="printer-select">{t("windowsPrinter")}
                          <select value={mapped} onChange={(e) => setConfig({ ...config, printer_mappings: { ...config.printer_mappings, [String(server.id)]: e.target.value } })}>
                            <option value="">{t("selectPrinter")}</option>
                            {localPrinters.map((local) => <option key={local.name} value={local.name}>{local.name}{local.is_default ? " · Default" : ""}</option>)}
                          </select>
                        </label>
                        <button className="icon-button" title={t("testPrint")} disabled={!mapped || busy === `test:${mapped}`} onClick={() => printTest(mapped)}><Printer size={18}/></button>
                      </div>
                    );
                  })}
                </div>
              )}
            </section>

            <section className="card controls-card">
              <div className="toggle-row">
                <div><strong>{t("cashDrawer")}</strong><span>ESC/POS</span></div>
                <button className={`switch ${config.open_drawer_after_print ? "on" : ""}`} onClick={() => setConfig({ ...config, open_drawer_after_print: !config.open_drawer_after_print })}><span /></button>
              </div>
              <label>{t("copies")}
                <select value={config.copies || 1} onChange={(e) => setConfig({ ...config, copies: Number(e.target.value) || 1 })}>
                  {[1, 2, 3].map((n) => <option key={n} value={n}>{n}</option>)}
                </select>
              </label>
              <div className="control-actions">
                <button className="secondary" onClick={save} disabled={busy === "save"}><Save size={18}/>{t("save")}</button>
                <button className={status.running ? "danger" : "primary"} onClick={toggleService} disabled={busy === "service"}>
                  {status.running ? <Square size={17}/> : <Play size={17}/>} {status.running ? t("stop") : t("start")}
                </button>
              </div>
              <p className="about-line">{t("company")} · {t("version")} {config.app_version}</p>
            </section>
          </>
        )}
      </section>
    </main>
  );
}

export default App;
