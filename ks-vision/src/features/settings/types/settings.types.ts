export interface WidgetSettings {
  opacity: number;
  width: number;
  height: number;
  alwaysOnTop: boolean;
  autoHideDuration: number;
  theme: 'dark' | 'light';
}

export interface AISettings {
  activeModel: string;
  temperature: number;
  maxTokens: number;
  streaming: boolean;
  responseLength: number;
  autoCopyResponse: boolean;
}

export interface VoiceSettings {
  inputDevice: string;
  pushToTalkShortcut: string;
  silenceTimeout: number;
}

export interface ScreenshotSettings {
  defaultCaptureMode: 'active' | 'full' | 'region' | 'scroll';
  ocrLanguage: string;
  imageQuality: number;
  scrollCaptureLimit: number;
}

export interface HotkeySettings {
  voice: string;
  screenshot: string;
  regionCapture: string;
  fullScreen: string;
  scrollCapture: string;
  toggleWidget: string;
  emergencyHide: string;
}

export interface AIProviderSettings {
  provider: string;
  apiKey: string;
}

export interface AppSettings {
  widget: WidgetSettings;
  ai: AISettings;
  voice: VoiceSettings;
  screenshot: ScreenshotSettings;
  hotkeys: HotkeySettings;
  startupMinimized?: boolean;
  launchOnStartup?: boolean;
  aiProvider?: AIProviderSettings;
}
