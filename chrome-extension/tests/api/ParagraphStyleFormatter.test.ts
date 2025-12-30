import { describe, it, expect } from 'vitest';
import { ParagraphStyleFormatter } from '@/api/ParagraphStyleFormatter';

describe('ParagraphStyleFormatter', () => {
  const formatter = new ParagraphStyleFormatter();

  describe('formatHeading', () => {
    it('should return HEADING_2 style batchUpdate request with Markdown prefix', () => {
      const result = formatter.formatHeading('Title');
      expect(result.text).toBe('## Title\n');

      const requests = result.createRequests(5);
      expect(requests).toHaveLength(2);
      expect(requests[0].insertText?.text).toBe('## Title\n');
      expect(requests[1].updateParagraphStyle?.paragraphStyle?.namedStyleType).toBe('HEADING_2');
      expect(requests[1].updateParagraphStyle?.range).toEqual({
        startIndex: 5,
        endIndex: 5 + '## Title\n'.length,
      });
    });

    it('should not double-prefix if already has ##', () => {
      const result = formatter.formatHeading('## Already Prefixed');
      expect(result.text).toBe('## Already Prefixed\n');
    });
  });

  describe('formatTimestamp', () => {
    it('should return timestamp text and gray textStyle request', () => {
      const date = new Date(2024, 0, 1, 12, 34, 56);
      const result = formatter.formatTimestamp(date);
      expect(result.text).toBe('[12:34:56] ');

      const requests = result.createRequests(10);
      expect(requests).toHaveLength(2);
      expect(requests[0].insertText?.text).toBe('[12:34:56] ');
      expect(requests[1].updateTextStyle?.textStyle?.foregroundColor).toBeTruthy();
      expect(requests[1].updateTextStyle?.range).toEqual({
        startIndex: 10,
        endIndex: 10 + '[12:34:56] '.length,
      });
    });
  });

  describe('formatSpeakerName', () => {
    it('should bold only the speaker name', () => {
      const result = formatter.formatSpeakerName('Alice');
      expect(result.text).toBe('Alice: ');

      const requests = result.createRequests(3);
      expect(requests).toHaveLength(2);
      expect(requests[0].insertText?.text).toBe('Alice: ');
      expect(requests[1].updateTextStyle?.textStyle?.bold).toBe(true);
      expect(requests[1].updateTextStyle?.range).toEqual({
        startIndex: 3,
        endIndex: 3 + 'Alice'.length,
      });
    });
  });

  describe('formatNormalText', () => {
    it('should append newline when missing', () => {
      const result = formatter.formatNormalText('Hello');
      expect(result.text).toBe('Hello\n');
      const requests = result.createRequests(1);
      expect(requests).toHaveLength(1);
      expect(requests[0].insertText?.text).toBe('Hello\n');
    });
  });

  describe('formatTranscriptLine', () => {
    it('should build combined text with timestamp and speaker', () => {
      const date = new Date(2024, 0, 1, 12, 34, 56);
      const result = formatter.formatTranscriptLine(date, 'Hello', {
        showTimestamp: true,
        showSpeaker: true,
        speaker: 'Bob',
      });

      expect(result.text).toBe('[12:34:56] Bob: Hello\n');
      const requests = result.createRequests(5);
      // 1 insertText + 1 timestamp style + 1 speaker style = 3
      expect(requests.length).toBeGreaterThanOrEqual(1);
      expect(requests[0].insertText?.text).toBe('[12:34:56] Bob: Hello\n');
    });

    it('should include style requests for timestamp and speaker', () => {
      const date = new Date(2024, 0, 1, 12, 34, 56);
      const result = formatter.formatTranscriptLine(date, 'Hello', {
        showTimestamp: true,
        showSpeaker: true,
        speaker: 'Bob',
      });

      const requests = result.createRequests(5);
      // insertText(1) + timestamp foregroundColor(1) + speaker bold(1) = 3
      expect(requests).toHaveLength(3);
      expect(requests[1].updateTextStyle?.textStyle?.foregroundColor).toBeTruthy();
      expect(requests[2].updateTextStyle?.textStyle?.bold).toBe(true);
    });
  });
});
