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

describe('renderMarkdown tables', () => {
  it('wraps tables so they scroll inside their own box', () => {
    const html = renderMarkdown('| a | b |\n|---|---|\n| 1 | 2 |');
    expect(html).toMatch(/^<div class="table-wrap"><table>/);
    expect(html).toContain('</table></div>');
  });

  it('does not touch table-like text inside attributes', () => {
    const html = renderMarkdown('<a href="#x" title="<table> onmouseover=alert(1)">t</a>');
    expect(html).not.toContain('table-wrap');
    const box = document.createElement('div');
    box.innerHTML = html;
    expect(box.querySelector('a')?.getAttributeNames().sort()).toEqual(['href', 'title']);
  });
});

describe('renderMarkdown table wrapping edge cases', () => {
  it('does not wrap "tables" inside svg or math, and keeps them inert', () => {
    const html = renderMarkdown(
      '<svg><table><tr><td><img src=x onerror="alert(1)"></td></tr></table></svg>\n\n<math><table><tr><td>m</td></tr></table></math>',
    );
    expect(html).not.toContain('onerror');
    const box = document.createElement('div');
    box.innerHTML = html;
    for (const w of box.querySelectorAll('.table-wrap')) {
      expect(w.closest('svg, math')).toBeNull();
    }
  });

  it('strips event handlers inside GFM table cells', () => {
    const html = renderMarkdown('| a | b |\n|---|---|\n| <img src=x onerror="alert(1)"> | 2 |');
    expect(html).toContain('table-wrap');
    expect(html).not.toContain('onerror');
  });
});
