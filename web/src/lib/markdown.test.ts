import { describe, expect, it } from 'vitest';
import { renderMarkdown } from './markdown';

describe('renderMarkdown sanitizing', () => {
  it('strips <style> tags and style attributes', () => {
    const html = renderMarkdown(
      '<style>body{display:none}</style>\n\n<p style="position:fixed">hi</p>',
    );
    expect(html).not.toContain('<style');
    expect(html).not.toContain('style=');
    expect(html).toContain('hi');
  });

  it('strips forms and their controls', () => {
    const html = renderMarkdown(
      '<form action="https://evil.example"><input name="q"><textarea></textarea><select><option>a</option></select><button>Go</button></form>',
    );
    for (const tag of ['<form', '<input', '<textarea', '<select', '<button']) {
      expect(html).not.toContain(tag);
    }
  });

  it('strips embeds', () => {
    const html = renderMarkdown(
      '<iframe src="https://e.com"></iframe><object data="x"></object><embed src="x">',
    );
    for (const tag of ['<iframe', '<object', '<embed']) expect(html).not.toContain(tag);
  });

  it('strips javascript: links', () => {
    const html = renderMarkdown('[click](javascript:alert(1)) <a href="javascript:alert(2)">x</a>');
    expect(html).not.toContain('javascript:');
    expect(html).toContain('click');
  });

  it('removes remote images but keeps their alt text', () => {
    const html = renderMarkdown(
      '![tracker](https://evil.example/pixel.png) <img src="//evil.example/p.png" alt="p2">',
    );
    expect(html).not.toContain('<img');
    expect(html).not.toContain('evil.example');
    expect(html).toContain('tracker');
    expect(html).toContain('p2');
  });

  it('keeps relative and data:image images', () => {
    const html = renderMarkdown('![a](diagram.png) ![b](data:image/png;base64,iVBORw0KGgo=)');
    expect(html).toContain('src="diagram.png"');
    expect(html).toContain('src="data:image/png;base64,iVBORw0KGgo="');
  });
});
