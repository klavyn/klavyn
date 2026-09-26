// klavyn — multi-language chord demo
// Lights up the keys of a word on that language's keyboard layout, then
// reveals the word. Cycles through languages; tabs jump to one.
(function () {
  var root = document.getElementById('kbd-demo');
  if (!root) return;

  // Keyboard layouts: three letter rows, legends in physical QWERTY positions.
  var LAYOUTS = {
    qwerty: ['qwertyuiop', 'asdfghjkl', 'zxcvbnm'].map(splitLatin),
    azerty: ['azertyuiop', 'qsdfghjklm', 'wxcvbn'].map(splitLatin),
    dubeolsik: [
      ['ㅂ','ㅈ','ㄷ','ㄱ','ㅅ','ㅛ','ㅕ','ㅑ','ㅐ','ㅔ'],
      ['ㅁ','ㄴ','ㅇ','ㄹ','ㅎ','ㅗ','ㅓ','ㅏ','ㅣ'],
      ['ㅋ','ㅌ','ㅊ','ㅍ','ㅠ','ㅜ','ㅡ']
    ],
    jcuken: [
      ['й','ц','у','к','е','н','г','ш','щ','з'],
      ['ф','ы','в','а','п','р','о','л','д'],
      ['я','ч','с','м','и','т','ь']
    ],
    arabic: [
      ['ض','ص','ث','ق','ف','غ','ع','ه','خ','ح'],
      ['ش','س','ي','ب','ل','ا','ت','ن','م'],
      ['ئ','ء','ؤ','ر','لا','ى','ة']
    ]
  };
  function splitLatin(s) { return s.split(''); }

  // Each demo: the word, the keys that make it (press order), a gloss, and dir.
  var DEMOS = [
    { tab: 'English', layout: 'qwerty',    word: 'chord',  keys: ['c','h','o','r','d'],       gloss: 'QWERTY · type the word\u2019s letters' },
    { tab: 'Français', layout: 'azerty',   word: 'merci',  keys: ['m','e','r','c','i'],       gloss: 'AZERTY · \u201cthank you\u201d' },
    { tab: '한국어',    layout: 'dubeolsik', word: '안녕',   keys: ['ㅇ','ㅏ','ㄴ','ㅕ'],         gloss: 'Dubeolsik · \u201chi\u201d (annyeong)' },
    { tab: 'Русский',  layout: 'jcuken',   word: 'привет', keys: ['п','р','и','в','е','т'],  gloss: '\u0419\u0426\u0423\u041a\u0415\u041d · \u201chi\u201d (privet)' },
    { tab: 'العربية',  layout: 'arabic',   word: 'سلام',   keys: ['س','ل','ا','م'],           gloss: 'Arabic · \u201cpeace\u201d (salām)', rtl: true }
  ];

  var reduce = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var idx = 0, timers = [], paused = false;

  // Build static shell
  var tabs = document.createElement('div'); tabs.className = 'kdemo-tabs';
  var stage = document.createElement('div'); stage.className = 'kdemo-stage';
  var kbd = document.createElement('div'); kbd.className = 'kbd';
  var out = document.createElement('div'); out.className = 'kbd-out';
  out.innerHTML = '<span class="cue">→</span><span class="word"></span><span class="gloss"></span>';
  stage.appendChild(kbd); stage.appendChild(out);

  DEMOS.forEach(function (d, i) {
    var b = document.createElement('button');
    b.className = 'ktab'; b.type = 'button'; b.textContent = d.tab;
    b.addEventListener('click', function () { select(i); });
    tabs.appendChild(b);
  });
  root.appendChild(tabs); root.appendChild(stage);

  function clearTimers() { timers.forEach(clearTimeout); timers = []; }
  function after(ms, fn) { timers.push(setTimeout(fn, ms)); }

  function renderKeyboard(layoutKey) {
    kbd.innerHTML = '';
    LAYOUTS[layoutKey].forEach(function (row) {
      var r = document.createElement('div'); r.className = 'kbd-row';
      row.forEach(function (legend) {
        var k = document.createElement('div');
        k.className = 'kkey'; k.setAttribute('data-legend', legend); k.textContent = legend;
        r.appendChild(k);
      });
      kbd.appendChild(r);
    });
  }

  function litKey(legend, alt) {
    var el = kbd.querySelector('.kkey[data-legend="' + cssEsc(legend) + '"]');
    if (el) { el.classList.add('lit'); if (alt) el.classList.add('alt'); }
  }
  function cssEsc(s) { return (window.CSS && CSS.escape) ? CSS.escape(s) : s.replace(/["\\]/g, '\\$&'); }

  function setActive(i) {
    tabs.querySelectorAll('.ktab').forEach(function (t, j) { t.classList.toggle('active', j === i); });
  }

  function show(i) {
    clearTimers();
    var d = DEMOS[i];
    setActive(i);
    renderKeyboard(d.layout);
    var word = out.querySelector('.word'), gloss = out.querySelector('.gloss');
    word.classList.remove('show'); gloss.classList.remove('show');
    word.textContent = d.word; word.setAttribute('dir', d.rtl ? 'rtl' : 'ltr');
    gloss.textContent = d.gloss;

    if (reduce) { // static: light all keys, show output immediately
      d.keys.forEach(function (lg, n) { litKey(lg, n % 2 === 1); });
      word.classList.add('show'); gloss.classList.add('show');
      return;
    }

    // animated: light keys one at a time (roll), then reveal the word
    var step = 260;
    d.keys.forEach(function (lg, n) { after(400 + n * step, function () { litKey(lg, n % 2 === 1); }); });
    var done = 400 + d.keys.length * step;
    after(done + 120, function () { word.classList.add('show'); });
    after(done + 260, function () { gloss.classList.add('show'); });
    if (!paused) after(done + 2100, function () { idx = (i + 1) % DEMOS.length; show(idx); });
  }

  function select(i) { paused = true; idx = i; show(i); } // manual pick stops auto-cycle

  // Kick off when scrolled into view (saves work, feels intentional)
  if ('IntersectionObserver' in window && !reduce) {
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) { if (e.isIntersecting) { show(idx); io.disconnect(); } });
    }, { threshold: 0.35 });
    io.observe(root);
  } else {
    show(idx);
  }
})();
