export const formatResponse = (text: string): string => {
  if (!text) return '';
  
  // Collapse 3 or more consecutive newlines into 2
  let cleaned = text.replace(/\n{3,}/g, '\n\n');
  
  // Trim trailing spaces on each line to preserve formatting and clean up whitespace
  cleaned = cleaned.split('\n').map(line => line.trimEnd()).join('\n');
  
  return cleaned.trim();
};

export const cleanMarkdownBlock = (text: string): string => {
  if (!text) return '';
  return text.replace(/^```[a-z]*\n/i, '').replace(/\n```$/, '');
};
