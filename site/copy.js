// klavyn — generic clipboard copy for any <button class="copy" data-copy="...">
(function () {
  document.querySelectorAll('.copy[data-copy]').forEach(function (btn) {
    btn.addEventListener('click', function () {
      if (!navigator.clipboard) return;
      navigator.clipboard.writeText(btn.getAttribute('data-copy')).then(function () {
        var prev = btn.textContent;
        btn.textContent = 'Copied';
        btn.classList.add('done');
        setTimeout(function () {
          btn.textContent = prev || 'Copy';
          btn.classList.remove('done');
        }, 1600);
      });
    });
  });
})();
