import { describe, expect, it } from 'vitest';
import { hl, norm, searchWords } from './search';

describe('search helpers', () => {
  it('norm strips diacritics per character and keeps the length', () => {
    expect(norm('Bảo mật')).toBe('bao mat');
    expect(norm('Đăng nhập')).toBe('dang nhap');
    expect(norm('đường')).toBe('duong');
    expect(norm('Ổn định').length).toBe('Ổn định'.length);
  });

  it('highlight_ignores_diacritics', () => {
    expect(hl('Bảo mật', ['bao'])).toBe('<mark>Bảo</mark> mật');
    expect(hl('Đăng nhập an toàn', searchWords('dang nhap'))).toBe(
      '<mark>Đăng</mark> <mark>nhập</mark> an toàn',
    );
    expect(hl('Đăng nhập', ['đăng'])).toBe('Đăng nhập'); // words must already be normalised
    expect(hl('Đăng nhập', searchWords('ĐĂNG'))).toBe('<mark>Đăng</mark> nhập');
  });

  it('hl marks every occurrence, merges adjacent words and escapes HTML', () => {
    expect(hl('token, Token', ['tok'])).toBe('<mark>tok</mark>en, <mark>Tok</mark>en');
    expect(hl('abc', ['ab', 'bc'])).toBe('<mark>abc</mark>');
    expect(hl('<b>&x', ['x'])).toBe('&lt;b&gt;&amp;<mark>x</mark>');
    expect(hl('plain', [])).toBe('plain');
  });

  it('searchWords normalises and splits on spaces', () => {
    expect(searchWords('  Bảo   MẬT ')).toEqual(['bao', 'mat']);
    expect(searchWords('   ')).toEqual([]);
  });
});
