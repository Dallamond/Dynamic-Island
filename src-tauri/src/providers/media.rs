//! Música vía SMTC (Global System Media Transport Controls): Spotify, navegadores con YouTube, etc.
//!
//! Funciona por eventos, sin sondeo: Windows avisa de cambios de sesión, canción, reproducción
//! y línea de tiempo; un hilo propio relee el estado y lo emite como `media://state`.
//! El frontend interpola la barra de progreso a partir de `positionMs` + `updatedAtMs`.

use base64::Engine;
use serde::Serialize;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::DataReader;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub app_id: String,
    pub app_name: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    pub position_ms: i64,
    pub duration_ms: i64,
    /// Instante (ms Unix) en que se midió `position_ms`.
    pub updated_at_ms: i64,
    pub can_prev: bool,
    pub can_next: bool,
    pub can_seek: bool,
    /// Carátula como data URL (solo cambia con la canción).
    pub thumbnail: Option<String>,
}

pub enum Msg {
    Refresh,
    Control(Control),
    Stop,
}

pub enum Control {
    PlayPause,
    Next,
    Prev,
    Seek(i64),
}

/// Canal hacia el hilo del proveedor; `None` si el módulo está desactivado.
#[derive(Default)]
pub struct MediaHandle(pub Mutex<Option<Sender<Msg>>>);

pub fn set_enabled(app: &AppHandle, enabled: bool) {
    let st = app.state::<MediaHandle>();
    let mut slot = st.0.lock().unwrap();
    match (enabled, slot.is_some()) {
        (true, false) => {
            let (tx, rx) = channel();
            let app2 = app.clone();
            let tx2 = tx.clone();
            std::thread::Builder::new()
                .name("media".into())
                .spawn(move || {
                    if let Err(e) = run(&app2, tx2, rx) {
                        eprintln!("[media] proveedor detenido: {e}");
                    }
                })
                .ok();
            *slot = Some(tx);
        }
        (false, true) => {
            if let Some(tx) = slot.take() {
                let _ = tx.send(Msg::Stop);
            }
            let _ = app.emit("media://state", None::<MediaState>);
        }
        _ => {}
    }
}

pub fn send(app: &AppHandle, msg: Msg) {
    if let Some(tx) = app.state::<MediaHandle>().0.lock().unwrap().as_ref() {
        let _ = tx.send(msg);
    }
}

/// Suscripciones de una sesión, para poder quitarlas al cambiar de sesiones.
struct SessionSubs {
    session: Session,
    tokens: [i64; 3],
}

fn run(app: &AppHandle, tx: Sender<Msg>, rx: Receiver<Msg>) -> windows::core::Result<()> {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let mgr = SessionManager::RequestAsync()?.join()?;
    let t1 = {
        let tx = tx.clone();
        mgr.SessionsChanged(&TypedEventHandler::new(move |_, _| {
            let _ = tx.send(Msg::Refresh);
            Ok(())
        }))?
    };
    let t2 = {
        let tx = tx.clone();
        mgr.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
            let _ = tx.send(Msg::Refresh);
            Ok(())
        }))?
    };

    let mut subs: Vec<SessionSubs> = Vec::new();
    let mut last: Option<MediaState> = None;
    let mut thumb_cache: (String, Option<String>) = (String::new(), None);
    resubscribe(&mgr, &tx, &mut subs);
    let _ = tx.send(Msg::Refresh);

    loop {
        // Espera con timeout: agrupa ráfagas de eventos (Spotify manda varios por cambio).
        match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Msg::Stop) => break,
            Ok(Msg::Control(c)) => {
                if let Some(s) = pick_session(&mgr) {
                    let _ = control(&s, c);
                }
                continue;
            }
            Ok(Msg::Refresh) => {
                std::thread::sleep(Duration::from_millis(60));
                let mut stop = false;
                while let Ok(m) = rx.try_recv() {
                    match m {
                        Msg::Stop => stop = true,
                        Msg::Control(c) => {
                            if let Some(s) = pick_session(&mgr) {
                                let _ = control(&s, c);
                            }
                        }
                        Msg::Refresh => {}
                    }
                }
                if stop {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        resubscribe(&mgr, &tx, &mut subs);
        let state = pick_session(&mgr).and_then(|s| read_state(&s, &mut thumb_cache).ok());
        if state != last {
            let _ = app.emit("media://state", state.clone());
            last = state;
        }
    }

    for s in subs.drain(..) {
        unsubscribe(s);
    }
    let _ = mgr.RemoveSessionsChanged(t1);
    let _ = mgr.RemoveCurrentSessionChanged(t2);
    Ok(())
}

/// Suscribe a los eventos de cada sesión (y quita las de sesiones que ya no existen).
fn resubscribe(mgr: &SessionManager, tx: &Sender<Msg>, subs: &mut Vec<SessionSubs>) {
    let Ok(list) = mgr.GetSessions() else { return };
    let sessions: Vec<Session> = list.into_iter().collect();
    let ids: Vec<String> = sessions.iter().map(|s| app_id(s)).collect();
    subs.retain(|s| {
        let keep = ids.contains(&app_id(&s.session));
        if !keep {
            unsubscribe(SessionSubs { session: s.session.clone(), tokens: s.tokens });
        }
        keep
    });
    for s in sessions {
        let id = app_id(&s);
        if subs.iter().any(|x| app_id(&x.session) == id) {
            continue;
        }
        let mk = || {
            let tx = tx.clone();
            move || {
                let _ = tx.send(Msg::Refresh);
            }
        };
        let (a, b, c) = (mk(), mk(), mk());
        let tokens = (|| -> windows::core::Result<[i64; 3]> {
            Ok([
                s.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
                    a();
                    Ok(())
                }))?,
                s.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
                    b();
                    Ok(())
                }))?,
                s.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
                    c();
                    Ok(())
                }))?,
            ])
        })();
        if let Ok(tokens) = tokens {
            subs.push(SessionSubs { session: s, tokens });
        }
    }
}

fn unsubscribe(s: SessionSubs) {
    let _ = s.session.RemoveMediaPropertiesChanged(s.tokens[0]);
    let _ = s.session.RemovePlaybackInfoChanged(s.tokens[1]);
    let _ = s.session.RemoveTimelinePropertiesChanged(s.tokens[2]);
}

fn app_id(s: &Session) -> String {
    s.SourceAppUserModelId().map(|h| h.to_string()).unwrap_or_default()
}

fn is_playing(s: &Session) -> bool {
    s.GetPlaybackInfo().and_then(|i| i.PlaybackStatus()).map(|st| st == Status::Playing).unwrap_or(false)
}

/// Sesión a mostrar: la que esté sonando (Spotify primero); si ninguna suena, la actual de Windows.
fn pick_session(mgr: &SessionManager) -> Option<Session> {
    let sessions: Vec<Session> = mgr.GetSessions().ok()?.into_iter().collect();
    let playing: Vec<&Session> = sessions.iter().filter(|s| is_playing(s)).collect();
    if let Some(s) = playing.iter().find(|s| app_id(s).to_lowercase().contains("spotify")) {
        return Some((*s).clone());
    }
    if let Some(s) = playing.first() {
        return Some((*s).clone());
    }
    mgr.GetCurrentSession().ok()
}

fn control(s: &Session, c: Control) -> windows::core::Result<bool> {
    match c {
        Control::PlayPause => s.TryTogglePlayPauseAsync()?.join(),
        Control::Next => s.TrySkipNextAsync()?.join(),
        Control::Prev => s.TrySkipPreviousAsync()?.join(),
        Control::Seek(ms) => s.TryChangePlaybackPositionAsync(ms * 10_000)?.join(),
    }
}

/// Diferencia entre la época de Windows (1601) y la Unix (1970), en unidades de 100 ns.
const EPOCH_DIFF: i64 = 116_444_736_000_000_000;

fn read_state(s: &Session, thumb_cache: &mut (String, Option<String>)) -> windows::core::Result<MediaState> {
    let props = s.TryGetMediaPropertiesAsync()?.join()?;
    let info = s.GetPlaybackInfo()?;
    let controls = info.Controls()?;
    let timeline = s.GetTimelineProperties()?;
    let id = app_id(s);
    let title = props.Title()?.to_string();
    let artist = props.Artist()?.to_string();

    let key = format!("{id}|{title}|{artist}");
    if thumb_cache.0 != key {
        *thumb_cache = (key, read_thumbnail(&props).ok().flatten());
    }

    let start = timeline.StartTime()?.Duration;
    let end = timeline.EndTime()?.Duration;
    let pos = timeline.Position()?.Duration;
    let updated = timeline.LastUpdatedTime()?.UniversalTime;
    Ok(MediaState {
        app_name: friendly_app_name(&id),
        app_id: id,
        title,
        artist,
        album: props.AlbumTitle().map(|h| h.to_string()).unwrap_or_default(),
        playing: info.PlaybackStatus()? == Status::Playing,
        position_ms: (pos - start).max(0) / 10_000,
        duration_ms: (end - start).max(0) / 10_000,
        updated_at_ms: if updated > 0 { (updated - EPOCH_DIFF) / 10_000 } else { 0 },
        can_prev: controls.IsPreviousEnabled()?,
        can_next: controls.IsNextEnabled()?,
        can_seek: controls.IsPlaybackPositionEnabled()?,
        thumbnail: thumb_cache.1.clone(),
    })
}

fn read_thumbnail(
    props: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties,
) -> windows::core::Result<Option<String>> {
    let Ok(thumb) = props.Thumbnail() else { return Ok(None) };
    let stream = thumb.OpenReadAsync()?.join()?;
    let size = stream.Size()? as u32;
    if size == 0 || size > 4_000_000 {
        return Ok(None);
    }
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
    reader.LoadAsync(size)?.join()?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf)?;
    let mime = stream.ContentType().map(|h| h.to_string()).unwrap_or_default();
    let mime = if mime.is_empty() { "image/jpeg".to_string() } else { mime };
    Ok(Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(buf))))
}

fn friendly_app_name(id: &str) -> String {
    let l = id.to_lowercase();
    let name = if l.contains("spotify") {
        "Spotify"
    } else if l.contains("chrome") {
        "Chrome"
    } else if l.contains("msedge") || l.contains("edge") {
        "Edge"
    } else if l.contains("firefox") {
        "Firefox"
    } else if l.contains("brave") {
        "Brave"
    } else if l.contains("opera") {
        "Opera"
    } else if l.contains("zunemusic") || l.contains("music") {
        "Multimedia"
    } else if l.contains("vlc") {
        "VLC"
    } else {
        return id.trim_end_matches(".exe").to_string();
    };
    name.to_string()
}
