import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from '../types/settings.types';
import { useAI } from '../../ai';
import { useSettings } from '../hooks/useSettings';
import { GlassCheckbox } from '../../../shared/components/glass';

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
      <div className="lg-stack">
        <div className="lg-section"><span>Parameters</span></div>

        <div className="lg-block">
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

        <div className="lg-block">
          <label className="ks-settings-label">Temp ({preferences.ai.temperature})</label>
          <input
            type="range"
            min="0"
            max="1.5"
            step="0.1"
            value={preferences.ai.temperature}
            onChange={(e) => updateAI('temperature', parseFloat(e.target.value))}
            className="lg-slider"
          />
        </div>

        <div className="lg-row">
          <span className="lg-row-label">Max Tokens</span>
          <input
            type="number"
            value={preferences.ai.maxTokens}
            onChange={(e) => updateAI('maxTokens', parseInt(e.target.value) || 128)}
            className="ks-settings-input"
          />
        </div>

        <div className="lg-row">
          <span className="lg-row-label">Streaming Mode</span>
          <GlassCheckbox
            checked={preferences.ai.streaming}
            onChange={(checked) => updateAI('streaming', checked)}
          />
        </div>

        <div className="lg-row">
          <span className="lg-row-label">Auto Copy Response</span>
          <GlassCheckbox
            checked={preferences.ai.autoCopyResponse}
            onChange={(checked) => updateAI('autoCopyResponse', checked)}
          />
        </div>
      </div>

      <div className="lg-stack mt-2">
        <div className="lg-section"><span>AI Provider</span></div>

        <div className="lg-block">
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
          <>
            <div className="lg-block">
              <label className="ks-settings-label">Gemini API Key</label>
              <div className="relative flex w-full items-center">
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
                  className="absolute right-1.5 px-1 text-[8px] font-bold uppercase text-[var(--lg-text-muted)] hover:text-[var(--lg-text)]"
                >
                  {showApiKey ? 'Hide' : 'Show'}
                </button>
              </div>
              {!hasExistingKey && (
                <div className="text-[11px] font-medium text-amber-300">
                  No Gemini API key configured.
                </div>
              )}
            </div>

            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                onClick={handleTestConnection}
                disabled={testing || !(preferences.aiProvider?.apiKey?.trim())}
                className="lg-btn h-8 px-3 text-[11px] disabled:opacity-50"
              >
                {testing ? 'Testing...' : 'Test Connection'}
              </button>
              <button
                type="button"
                onClick={handleSaveApiKey}
                disabled={saving || !(preferences.aiProvider?.apiKey?.trim())}
                className="lg-btn lg-btn--primary h-8 px-3 text-[11px] disabled:opacity-50"
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
                  className="lg-btn h-8 px-3 text-[11px]"
                >
                  Cancel
                </button>
              )}
            </div>
          </>
        ) : (
          <>
            <div className="lg-block">
              <div className="lg-status px-0">
                <span className="lg-status-dot" />
                API Key Configured
              </div>
              <div className="text-[11px] leading-normal text-[var(--lg-text-secondary)]">
                A Gemini API key has already been securely saved for this application.
              </div>
              <div className="font-mono text-[10px] tracking-wider text-[var(--lg-text-muted)]">
                ••••••••••••••••••••••••••••••••
              </div>
            </div>

            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                onClick={handleTestConnection}
                disabled={testing}
                className="lg-btn h-8 px-3 text-[11px]"
              >
                {testing ? 'Testing...' : 'Test Connection'}
              </button>
              <button
                type="button"
                onClick={() => {
                  setIsEditing(true);
                  setStatusMessage('');
                }}
                className="lg-btn lg-btn--primary h-8 px-3 text-[11px]"
              >
                Update API Key
              </button>
            </div>
          </>
        )}

        {statusMessage && (
          <div className={`text-[11px] font-semibold ${
            statusMessage.includes('successful') || statusMessage.includes('Configured')
              ? 'text-[var(--lg-success)]'
              : 'text-[var(--lg-danger)]'
          }`}>
            {statusMessage}
          </div>
        )}
      </div>
    </div>
  );
};
