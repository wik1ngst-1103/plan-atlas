(() => {
  'use strict';

  const VI = {
    title: 'Plan Atlas: gom plan của Claude Code về một chỗ',
    description: 'Plan Atlas gom các plan Claude Code viết ở mọi dự án, trình bày thành tài liệu có danh sách việc theo dõi được, và cho cả nhóm biết Claude đang làm gì bằng lời dễ hiểu. Miễn phí, mã nguồn mở, chạy trên máy bạn.',
    skip: 'Bỏ qua, đến nội dung chính',
    navLabel: 'Điều hướng trang',
    langLabel: 'Ngôn ngữ',
    navInstall: 'Cài đặt',
    heroTitle: 'Mọi plan Claude viết, về chung một chỗ.',
    heroLead: 'Đọc plan như một tài liệu, theo dõi từng việc và xem Claude đang làm gì, ngay lúc này.',
    heroInstall: 'Cài Plan Atlas',
    heroGithub: 'Xem mã nguồn trên GitHub',
    heroTerms: 'Miễn phí, mã nguồn mở (MIT). Chạy ngay trên máy bạn: macOS, Linux và Windows.',
    ovTitle: 'Mọi dự án trên một màn hình',
    ovLead: 'Plan từ mọi repo, xếp theo dự án, tiến độ cập nhật ngay trên từng thẻ.',
    ovAlt: 'Trang tổng quan của Plan Atlas: số plan và số việc, rồi các thẻ plan xếp theo dự án, một thẻ cho thấy Claude đang làm một việc cùng hai trợ lý.',
    f1t: 'Tự tìm plan có sẵn',
    f1d: 'Plan trong <code>~/.claude/plans</code> được nhập ngay lần chạy đầu.',
    f2t: 'Nhập nốt phần còn lại',
    f2d: 'Nhập file hay thư mục Markdown bất kỳ.',
    f3t: 'Không đụng file gốc',
    f3d: 'File gốc không bao giờ bị sửa.',
    phoneCap: 'Cùng plan đó trong một cửa sổ hẹp.',
    phoneAlt: 'Plan Atlas trong cửa sổ hẹp: plan Migrate sessions to JWT + RBAC với trạng thái, tiến độ và chủ đề.',
    ovNote: 'Ảnh chụp trên trang này dùng plan mẫu, không phải dự án thật.',
    stTitle: 'Năm cách xem một plan',
    st1: 'Tài liệu',
    st2: 'Danh sách việc',
    st3: 'Bảng việc',
    st4: 'Bản đồ',
    st5: 'Tìm kiếm',
    docAlt: 'Một plan hiển thị dạng tài liệu: tiêu đề, trạng thái, các plan liên quan, rồi các phần đánh số, mục lục bên phải.',
    docT: 'Plan đọc như một tài liệu',
    docD: 'Năm phần cố định, từ tổng quan đến phương án rút lui, mục lục bám theo chỗ bạn đọc.',
    tasksAlt: 'Danh sách việc của một plan: mỗi việc có trạng thái, độ khó từ 1 đến 5 và model Claude sẽ dùng.',
    tasksT: 'Việc nào cũng ghi rõ độ khó',
    tasksD: 'Độ phức tạp từ 1 đến 5 quyết định model (Haiku, Sonnet hay Opus) và mức effort.',
    boardAlt: 'Bảng công việc với các cột chưa làm, đang làm, chờ duyệt và xong, chứa thẻ việc từ nhiều plan.',
    boardT: 'Kéo thẻ để đổi trạng thái',
    boardD: 'Một bảng cho việc của mọi plan: chưa làm, đang làm, chờ duyệt, xong.',
    mapAlt: 'Bản đồ kiến thức: danh sách chủ đề bên cạnh đồ thị các plan, nối bằng mũi tên phụ thuộc và đường chủ đề chung.',
    mapT: 'Thấy ngay plan nào cần plan nào',
    mapD: 'Chủ đề lấy từ một bộ phân loại chung, và đồ thị nối các plan dựa trên nhau.',
    searchAlt: 'Ô tìm kiếm đang mở, kết quả chia thành plan, việc và chủ đề, phần khớp được tô sáng.',
    searchT: 'Tìm kiếm không cần gõ dấu',
    searchD: 'Plan, việc và chủ đề trong cùng một ô. Gõ “bao mat” vẫn ra “bảo mật”.',
    teamTitle: 'Cả nhóm đều đọc hiểu',
    teamLead: 'Việc Claude và các trợ lý của nó đang làm được viết thành câu dễ hiểu, xếp đúng dưới plan và việc.',
    thRan: 'Claude chạy',
    thReads: 'Cả nhóm đọc',
    r1: 'Trợ lý lập trình đã đọc refresh.rs',
    r2: 'Trợ lý tìm hiểu đã tìm kiếm trong code',
    r3: 'Trợ lý lập trình đã sửa refresh.rs',
    r4: 'Trợ lý lập trình đã chạy kiểm thử',
    teamNote: 'Lập trình viên vẫn mở được lệnh gốc chỉ với một cú bấm.',
    howTitle: 'Một lệnh để nối với Claude Code',
    copy: 'Chép',
    copied: 'Đã chép',
    copiedStatus: 'Đã chép lệnh vào bộ nhớ tạm.',
    copyFail: 'Không chép được. Hãy bôi đen lệnh và chép thủ công.',
    thPiece: 'Lệnh sẽ thêm',
    thDoes: 'Để',
    thWhere: 'Ở đâu',
    p1: 'Một Skill',
    p1d: 'Claude biết viết plan ở đâu và theo cấu trúc nào',
    p2: 'Hook',
    p2d: 'Plan Atlas thấy Claude và từng trợ lý đang làm gì',
    p2w: '(sao lưu trước khi sửa)',
    p3: 'Một MCP server',
    p3d: 'Claude tạo plan, liên kết plan liên quan và cập nhật việc',
    howNote: '<code>plan-atlas uninstall-claude</code> gỡ cả ba, các thiết lập khác của bạn vẫn giữ nguyên.',
    prTitle: 'Plan của bạn không rời khỏi máy',
    pl1: 'Một file thực thi gồm server, MCP, hook và giao diện',
    pl2: 'Bộ nhớ lúc rảnh khi đã lập chỉ mục 1.000 plan',
    pl3: 'Địa chỉ duy nhất nó lắng nghe',
    c1: 'Không thu thập dữ liệu sử dụng, không kết nối ra ngoài.',
    c2: 'Request phải gọi đúng server cục bộ, chặn được DNS rebinding.',
    c3: 'Hook xong trong vòng 200 ms, không bao giờ bắt Claude phải chờ.',
    inTitle: 'Cài bằng một dòng lệnh',
    inLead: 'Cài toàn cục, rồi chạy <code>plan-atlas</code>. Giao diện mở ở <code>127.0.0.1:7420</code>.',
    platform: 'Nền tảng',
    inNote: 'Bộ cài được phát hành kèm mỗi bản trên GitHub, bắt đầu từ v0.1.0. Muốn tự biên dịch? <a href="https://github.com/wik1ngst-1103/plan-atlas/blob/main/README.vi.md#cài-đặt">Build từ mã nguồn</a>.',
    faqTitle: 'Trước khi cài',
    q1: 'Plan của tôi có bị gửi đi đâu không?',
    a1: 'Không. Server chỉ lắng nghe ở 127.0.0.1, không thu thập dữ liệu sử dụng và không kết nối ra ngoài. Plan nằm dưới dạng file Markdown trong <code>~/.plan-atlas</code>.',
    q2: 'Chạy được trên hệ điều hành nào?',
    a2: 'macOS, Linux và Windows, chỉ với một file thực thi duy nhất.',
    q3: 'Có mất phí không?',
    a3: 'Không. Plan Atlas là mã nguồn mở theo giấy phép MIT, không cần tài khoản, không có gói trả phí.',
    q4: 'Đồng nghiệp có mở được từ máy của họ không?',
    a4: 'Hiện chưa: ứng dụng chỉ phục vụ chính máy đang chạy nó. Bạn có thể chia sẻ màn hình hoặc ngồi xem cùng; tính năng xuất plan thành trang tĩnh để chia sẻ đã có trong lộ trình.',
    q5: 'Tôi có phải đổi cách làm việc với Claude không?',
    a5: 'Không. Sau khi chạy <code>plan-atlas setup</code>, Claude tự ghi plan qua Plan Atlas. Plan viết ở plan mode được đăng ký ngay khi bạn duyệt.',
    q6: 'Gỡ cài đặt thế nào?',
    a6: 'Chạy <code>plan-atlas uninstall-claude</code> để gỡ Skill, hook và MCP, rồi gỡ file thực thi theo cách bạn đã cài. Dữ liệu nằm trong <code>~/.plan-atlas</code>; xoá thư mục này nếu không cần nữa.',
    footText: 'Phần mềm tự do theo giấy phép MIT, phát triển công khai trên GitHub.',
    fSec: 'Bảo mật',
    fLic: 'Giấy phép MIT',
    ovImg: 'assets/shots/overview-vi.webp',
    ovDark: 'assets/shots/overview-vi.webp'
  };

  const EN_EXTRA = { copy: 'Copy', copied: 'Copied', copiedStatus: 'Command copied to the clipboard.', copyFail: 'Could not copy. Select the command and copy it by hand.' };
  const root = document.documentElement;
  const EN = Object.assign({ title: document.title, description: '' }, EN_EXTRA);
  const metaDesc = document.querySelector('meta[name="description"]');
  EN.description = metaDesc ? metaDesc.content : '';

  // Remember the English source so switching back needs no second dictionary.
  document.querySelectorAll('[data-i18n]').forEach((el) => { EN[el.dataset.i18n] = el.innerHTML; });
  document.querySelectorAll('[data-i18n-attr]').forEach((el) => {
    el.dataset.i18nAttr.split(';').forEach((pair) => { const [attr, key] = pair.split(':'); EN[key] = el.getAttribute(attr); });
  });
  document.querySelectorAll('[data-i18n-src]').forEach((el) => { EN[el.dataset.i18nSrc] = el.getAttribute('src'); });
  document.querySelectorAll('[data-i18n-srcset]').forEach((el) => { EN[el.dataset.i18nSrcset] = el.getAttribute('srcset'); });

  let lang = 'en';
  const t = (key) => (lang === 'vi' && VI[key] != null ? VI[key] : EN[key]);

  function apply(next) {
    lang = next === 'vi' ? 'vi' : 'en';
    root.lang = lang;
    document.title = t('title');
    if (metaDesc) metaDesc.content = t('description');
    document.querySelectorAll('[data-i18n]').forEach((el) => { const v = t(el.dataset.i18n); if (v != null) el.innerHTML = v; });
    document.querySelectorAll('[data-i18n-attr]').forEach((el) => {
      el.dataset.i18nAttr.split(';').forEach((pair) => { const [attr, key] = pair.split(':'); const v = t(key); if (v != null) el.setAttribute(attr, v); });
    });
    document.querySelectorAll('[data-i18n-src]').forEach((el) => { el.setAttribute('src', t(el.dataset.i18nSrc)); });
    document.querySelectorAll('[data-i18n-srcset]').forEach((el) => { el.setAttribute('srcset', t(el.dataset.i18nSrcset)); });
    document.querySelectorAll('[data-lang-only]').forEach((el) => { el.hidden = el.dataset.langOnly !== lang; });
    document.querySelectorAll('.lang__btn').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.lang === lang)));
  }

  let saved = null;
  try { saved = localStorage.getItem('plan-atlas-site-lang'); } catch (e) { saved = null; }
  const preferred = saved || ((navigator.languages && navigator.languages[0]) || navigator.language || 'en');
  apply(/^vi\b/i.test(preferred) ? 'vi' : 'en');

  document.querySelectorAll('.lang__btn').forEach((b) => b.addEventListener('click', () => {
    apply(b.dataset.lang);
    try { localStorage.setItem('plan-atlas-site-lang', lang); } catch (e) { /* storage unavailable: keep the choice for this visit only */ }
  }));

  // Copy buttons
  const status = document.getElementById('copy-status');
  async function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) { await navigator.clipboard.writeText(text); return; }
    const ta = document.createElement('textarea');
    ta.value = text; ta.setAttribute('readonly', ''); ta.style.position = 'fixed'; ta.style.opacity = '0';
    document.body.appendChild(ta); ta.select();
    const ok = document.execCommand('copy');
    ta.remove();
    if (!ok) throw new Error('copy failed');
  }
  document.querySelectorAll('[data-copy]').forEach((btn) => btn.addEventListener('click', async () => {
    const src = document.getElementById(btn.dataset.copy);
    if (!src) return;
    try {
      await copyText(src.textContent.trim());
      btn.textContent = t('copied');
      status.textContent = t('copiedStatus');
      setTimeout(() => { btn.textContent = t('copy'); }, 1800);
    } catch (e) {
      status.textContent = t('copyFail');
    }
  }));

  // Platform tabs (without script every panel shows in order)
  document.querySelectorAll('[data-tabs]').forEach((box) => {
    const tabs = Array.from(box.querySelectorAll('[role="tab"]'));
    const select = (tab, focus) => {
      tabs.forEach((x) => {
        const on = x === tab;
        x.setAttribute('aria-selected', String(on));
        x.tabIndex = on ? 0 : -1;
        document.getElementById(x.getAttribute('aria-controls')).hidden = !on;
      });
      if (focus) tab.focus();
    };
    const ua = navigator.userAgent || '';
    select(/Windows/i.test(ua) ? tabs[1] : tabs[0], false);
    tabs.forEach((tab, i) => {
      tab.addEventListener('click', () => select(tab, false));
      tab.addEventListener('keydown', (e) => {
        let j = null;
        if (e.key === 'ArrowRight') j = (i + 1) % tabs.length;
        else if (e.key === 'ArrowLeft') j = (i - 1 + tabs.length) % tabs.length;
        else if (e.key === 'Home') j = 0;
        else if (e.key === 'End') j = tabs.length - 1;
        if (j !== null) { e.preventDefault(); select(tabs[j], true); }
      });
    });
  });

  // Feature stack: mark the view in reading position as the current stop
  const links = Array.from(document.querySelectorAll('.stack__index a'));
  if (links.length && 'IntersectionObserver' in window) {
    const byId = new Map(links.map((a) => [a.getAttribute('href').slice(1), a]));
    const io = new IntersectionObserver((entries) => {
      entries.forEach((en) => {
        if (!en.isIntersecting) return;
        links.forEach((a) => a.removeAttribute('aria-current'));
        const a = byId.get(en.target.id);
        if (a) a.setAttribute('aria-current', 'true');
      });
    }, { rootMargin: '-45% 0px -50% 0px' });
    byId.forEach((_, id) => { const el = document.getElementById(id); if (el) io.observe(el); });
  }
})();
