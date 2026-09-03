import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from '../types/settings.types';
import { useAI } from '../../ai';
import { useSettings } from '../hooks/useSettings';

interface SubProps {
  preferences: AppSettings;
  onChange: (pref: AppSettings) => void;
}

export const AISettingsView: React.FC<SubProps> = ({ preferences, onChange }) => {
  const { models } = useAI();
  const { saveSettings } = useSettings();

  const [showApiKey, setShowApiKey] = useState(false);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [statusMessage, setStatusMessage] = useState('');

  const hasExistingKey = !!preferences.aiProvider?.apiKey?.trim();
  const [isEditing, setIsEditing] = useState(!hasExistingKey);

  // Set isEditing to true if there is no key stored
  useEffect(() => {
    if (!hasExistingKey) {
      setIsEditing(true);
    }
  }, [hasExistingKey]);

  const updateAI = (key: string, value: any) => {
    onChange({
      ...preferences,
      ai: {
        ...preferences.ai,
        [key]: value,
      },
    });
  };

  const updateProvider = (key: string, value: any) => {
    onChange({
      ...preferences,
      aiProvider: {
        provider: preferences.aiProvider?.provider || 'Gemini',
        apiKey: preferences.aiProvider?.apiKey || '',
        [key]: value,
      },
    });
  };

  const handleSaveApiKey = async () => {
    const key = preferences.aiProvider?.apiKey?.trim();
    if (!key) {
      setStatusMessage('API Key cannot be empty.');
      return;
    }
    setSaving(true);
    setStatusMessage('Validating API Key...');
    try {
      // Validate key first
      const validationRes = await invoke<string>('test_gemini_connection', { apiKey: key });
      if (validationRes !== 'Connected successfully.') {
        setStatusMessage(validationRes);
        setSaving(false);
        return;
      }

      const updatedPrefs = {
        ...preferences,
        aiProvider: {
          provider: preferences.aiProvider?.provider || 'Gemini',
          apiKey: key,
        },
      };
      onChange(updatedPrefs);
      await saveSettings(updatedPrefs);
      setStatusMessage('API Key saved successfully.');
      setIsEditing(false); // Switch back to configured view
    } catch (err: any) {
      setStatusMessage(typeof err === 'string' ? err : 'Invalid API Key.');
    } finally {
      setSaving(false);
    }
  };

  const handleTestConnection = async () => {
    const key = preferences.aiProvider?.apiKey?.trim();
    if (!key) {
      setStatusMessage('API Key cannot be empty.');
      return;
    }
    setTesting(true);
    setStatusMessage('');
    try {
      const res = await invoke<string>('test_gemini_connection', { apiKey: key });
      setStatusMessage(res);
    } catch (err: any) {
      setStatusMessage(typeof err === 'string' ? err : 'Invalid API Key.');
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="ks-settings-page">
      <div className="ks-settings-card">
        <h4 className="text-[12px] text-cyan-300 font-semibold uppercase tracking-wide">Parameters</h4>
        
        <div className="flex flex-col gap-0.5">
          <label className="ks-settings-label">Active Model</label>
          <select
            value={preferences.ai.activeModel}
            onChange={(e) => updateAI('activeModel', e.target.value)}
            className="ks-settings-input cursor-pointer"
          >
            {models.length > 0 ? (
              models.map((m: any) => (
                <option key={m.model || m.name} value={m.model || m.name}>
                  {m.name || m.model}
                </option>
              ))
            ) : (
              <option value={preferences.ai.activeModel}>{preferences.ai.activeModel}</option>
            )}
          </select>
        </div>

        <div className="grid grid-cols-2 gap-1.5">
          <div className="flex flex-col gap-0.5">
            <label className="ks-settings-label">Temp ({preferences.ai.temperature})</label>
            <input
              type="range"
              min="0"
              max="1.5"
              step="0.1"
              value={preferences.ai.temperature}
              onChange={(e) => updateAI('temperature', parseFloat(e.target.value))}
              className="accent-cyan-500 cursor-pointer h-1.5"
            />
          </div>
          <div className="flex flex-col gap-0.5">
            <label className="ks-settings-label">Max Tokens</label>
            <input
              type="number"
              value={preferences.ai.maxTokens}
              onChange={(e) => updateAI('maxTokens', parseInt(e.target.value) || 128)}
              className="ks-settings-input"
            />
          </div>
        </div>

        <label className="ks-settings-row cursor-pointer">
          <span>Streaming Mode</span>
          <input 
            type="checkbox"
            checked={preferences.ai.streaming}
            onChange={(e) => updateAI('streaming', e.target.checked)}
            className="accent-cyan-500 h-3.5 w-3.5"
          />
        </label>

        <label className="ks-settings-row cursor-pointer">
          <span>Auto Copy Response</span>
          <input 
            type="checkbox"
            checked={preferences.ai.autoCopyResponse}
            onChange={(e) => updateAI('autoCopyResponse', e.target.checked)}
            className="accent-cyan-500 h-3.5 w-3.5"
          />
        </label>
      </div>

      {/* AI Provider Card */}
      <div className="ks-settings-card">
        <h4 className="text-[12px] text-cyan-300 font-semibold uppercase tracking-wide">AI Provider</h4>
        
        <div className="flex flex-col gap-0.5">
          <label className="ks-settings-label">Provider</label>
          <select
            value={preferences.aiProvider?.provider || 'Gemini'}
            onChange={(e) => updateProvider('provider', e.target.value)}
            disabled={!isEditing}
            className="ks-settings-input cursor-pointer disabled:opacity-50"
          >
            <option value="Gemini">Gemini</option>
          </select>
        </div>

        {isEditing ? (
          /* Editable Input Form */
          <>
            <div className="flex flex-col gap-0.5">
              <label className="ks-settings-label">Gemini API Key</label>
              <div className="relative flex items-center w-full">
                <input
                  type={showApiKey ? 'text' : 'password'}
                  value={preferences.aiProvider?.apiKey || ''}
                  onChange={(e) => updateProvider('apiKey', e.target.value)}
                  className="ks-settings-input pr-14"
                  placeholder="Enter Gemini API Key..."
                />
                <button
                  type="button"
                  onClick={() => setShowApiKey(!showApiKey)}
                  className="absolute right-1.5 text-slate-400 hover:text-slate-200 px-1 text-[8px] font-bold uppercase"
                >
                  {showApiKey ? 'Hide' : 'Show'}
                </button>
              </div>
            </div>

            {!hasExistingKey && (
              <div className="text-[9px] text-amber-500 font-bold mt-0.5">
                No Gemini API key configured.
              </div>
            )}

            <div className="flex gap-1.5 mt-1">
              <button
                type="button"
                onClick={handleTestConnection}
                disabled={testing || !(preferences.aiProvider?.apiKey?.trim())}
                className="px-2 py-0.5 bg-slate-950/50 hover:bg-slate-900 border border-slate-800/60 rounded text-[9px] text-slate-300 font-bold active:scale-95 transition-all disabled:opacity-50"
              >
                {testing ? 'Testing...' : 'Test Connection'}
              </button>
              <button
                type="button"
                onClick={handleSaveApiKey}
                disabled={saving || !(preferences.aiProvider?.apiKey?.trim())}
                className="px-2.5 py-0.5 bg-cyan-950 hover:bg-cyan-900 border border-cyan-800/50 rounded text-[9px] text-cyan-300 font-bold active:scale-95 transition-all disabled:opacity-50"
              >
                {saving ? 'Saving...' : 'Save'}
              </button>
              {hasExistingKey && (
                <button
                  type="button"
                  onClick={() => {
                    setIsEditing(false);
                    setStatusMessage('');
                  }}
                  className="px-2 py-0.5 bg-slate-950/50 hover:bg-slate-900 border border-slate-800/60 rounded text-[9px] text-slate-400 font-bold active:scale-95 transition-all"
                >
                  Cancel
                </button>
              )}
            </div>
          </>
        ) : (
          /* Secure Configured Display View */
          <>
            <div className="flex flex-col gap-1 border border-green-800/30 rounded p-1.5 bg-green-950/10">
              <div className="text-[9px] text-green-400 font-bold flex items-center gap-1">
                <span>✓</span> Gemini API Key Configured
              </div>
              <div className="text-[8px] text-slate-400 leading-normal">
                A Gemini API key has already been securely saved for this application.
              </div>
              <div className="text-[9px] text-slate-500 font-mono tracking-wider mt-0.5">
                ••••••••••••••••••••••••••••••••
              </div>
            </div>

            <div className="flex gap-1.5 mt-1">
              <button
                type="button"
                onClick={handleTestConnection}
                disabled={testing}
                className="px-2 py-0.5 bg-slate-950/50 hover:bg-slate-900 border border-slate-800/60 rounded text-[9px] text-slate-300 font-bold active:scale-95 transition-all"
              >
                {testing ? 'Testing...' : 'Test Connection'}
              </button>
              <button
                type="button"
                onClick={() => {
                  setIsEditing(true);
                  setStatusMessage('');
                }}
                className="px-2.5 py-0.5 bg-cyan-950 hover:bg-cyan-900 border border-cyan-800/50 rounded text-[9px] text-cyan-300 font-bold active:scale-95 transition-all"
              >
                Update API Key
              </button>
            </div>
          </>
        )}
        
        {statusMessage && (
          <div className={`text-[9px] mt-0.5 font-bold ${
            statusMessage.includes('successful') || statusMessage.includes('Configured') ? 'text-green-400' : 'text-rose-400'
          }`}>
            {statusMessage}
          </div>
        )}
      </div>
    </div>
  );
};
