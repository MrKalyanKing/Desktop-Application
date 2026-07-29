pub fn get_default_settings() -> &'static str {
    r#"{
        "widget": {
            "opacity": 90,
            "width": 400,
            "height": 450,
            "alwaysOnTop": true,
            "autoHideDuration": 0,
            "theme": "dark"
        },
        "ai": {
            "activeModel": "gemini-3.5-flash-lite",
            "temperature": 0.7,
            "maxTokens": 1024,
            "streaming": true,
            "responseLength": 80,
            "autoCopyResponse": true
        },
        "voice": {
            "inputDevice": "default",
            "pushToTalkShortcut": "Space",
            "silenceTimeout": 2000
        },
        "screenshot": {
            "defaultCaptureMode": "region",
            "ocrLanguage": "eng",
            "imageQuality": 90,
            "scrollCaptureLimit": 15
        },
        "hotkeys": {
            "voice": "Ctrl+Shift+V",
            "screenshot": "Ctrl+Shift+W",
            "regionCapture": "Ctrl+Shift+R",
            "fullScreen": "Ctrl+Shift+F",
            "scrollCapture": "Ctrl+Shift+S",
            "toggleWidget": "Ctrl+Shift+H",
            "emergencyHide": "Escape"
        }
    }"#
}
