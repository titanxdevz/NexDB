document.addEventListener('DOMContentLoaded', function () {
  
  // ─── 1. Global Language Selection & Sync ───
  var DEFAULT_LANG = 'rust';
  var currentLang = localStorage.getItem('nexdb-docs-lang') || DEFAULT_LANG;

  // Set active language globally
  function setLanguage(lang) {
    currentLang = lang;
    localStorage.setItem('nexdb-docs-lang', lang);

    // Update all tab buttons (both global pickers and code tabs)
    document.querySelectorAll('.nav-lang-btn, .docs-lang-option, .code-tab-btn').forEach(function (btn) {
      if (btn.dataset.lang === lang) {
        btn.classList.add('active');
      } else {
        btn.classList.remove('active');
      }
    });

    // Update all code tab contents
    document.querySelectorAll('.code-tab-content').forEach(function (content) {
      if (content.dataset.lang === lang) {
        content.classList.add('active');
      } else {
        content.classList.remove('active');
      }
    });
  }

  // Initialize language on load
  setLanguage(currentLang);

  // Add click listeners to all language selector buttons
  document.body.addEventListener('click', function (e) {
    var btn = e.target.closest('.nav-lang-btn, .docs-lang-option, .code-tab-btn');
    if (btn && btn.dataset.lang) {
      setLanguage(btn.dataset.lang);
    }
  });


  // ─── 2. Copy Code to Clipboard ───
  document.querySelectorAll('.copy-code-btn').forEach(function (btn) {
    btn.addEventListener('click', function () {
      var codeTabs = this.closest('.code-tabs, .hero-code');
      if (!codeTabs) return;

      // Find the active code block text
      var activeContent = codeTabs.querySelector('.code-tab-content.active code, code');
      if (!activeContent) return;

      var textToCopy = activeContent.innerText || activeContent.textContent;
      
      var self = this;
      navigator.clipboard.writeText(textToCopy).then(function () {
        self.classList.add('copied');
        var originalHTML = self.innerHTML;
        // Checkmark icon
        self.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>';
        
        setTimeout(function () {
          self.classList.remove('copied');
          self.innerHTML = originalHTML;
        }, 2000);
      }).catch(function (err) {
        console.error('Failed to copy text: ', err);
      });
    });
  });


  // ─── 3. Regex Syntax Highlighter ───
  function escapeHtml(text) {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  }

  function highlightRust(code) {
    return code
      // Comments
      .replace(/(\/\/.*)/g, '<span class="hl-comment">$1</span>')
      // Strings
      .replace(/(".*?")/g, '<span class="hl-string">$1</span>')
      .replace(/(r#".*?"#)/g, '<span class="hl-string">$1</span>')
      // Numbers
      .replace(/\b(\d+)\b/g, '<span class="hl-number">$1</span>')
      // Keywords
      .replace(/\b(let|fn|const|await|async|use|pub|struct|impl|match|return|mut|if|else|loop|for|in|type|dyn|static|as|mod|crate)\b/g, '<span class="hl-keyword">$1</span>')
      // Types
      .replace(/\b(NexDb|Collection|Document|QueryBuilder|Result|Option|Value|String|Vec|u16|u32|u64|usize|bool|char)\b/g, '<span class="hl-type">$1</span>')
      // Functions
      .replace(/\b(\w+)(?=\s*\()/g, '<span class="hl-fn">$1</span>')
      // Macros
      .replace(/\b(\w+!)/g, '<span class="hl-fn">$1</span>');
  }

  function highlightPython(code) {
    return code
      // Comments
      .replace(/(#.*)/g, '<span class="hl-comment">$1</span>')
      // Strings
      .replace(/(".*?")/g, '<span class="hl-string">$1</span>')
      .replace(/('.*?')/g, '<span class="hl-string">$1</span>')
      // Numbers
      .replace(/\b(\d+)\b/g, '<span class="hl-number">$1</span>')
      // Keywords
      .replace(/\b(def|class|import|from|as|await|async|return|if|else|elif|for|in|try|except|raise|with|pass|self|None|True|False)\b/g, '<span class="hl-keyword">$1</span>')
      // Types
      .replace(/\b(NexDb|dict|list|str|int|bool|float|Exception|RuntimeError)\b/g, '<span class="hl-type">$1</span>')
      // Functions
      .replace(/\b(\w+)(?=\s*\()/g, '<span class="hl-fn">$1</span>');
  }

  function highlightJavascript(code) {
    return code
      // Comments
      .replace(/(\/\/.*)/g, '<span class="hl-comment">$1</span>')
      // Strings
      .replace(/(".*?")/g, '<span class="hl-string">$1</span>')
      .replace(/('.*?')/g, '<span class="hl-string">$1</span>')
      .replace(/(`.*?`)/g, '<span class="hl-string">$1</span>')
      // Numbers
      .replace(/\b(\d+)\b/g, '<span class="hl-number">$1</span>')
      // Keywords
      .replace(/\b(const|let|var|function|class|import|export|from|require|await|async|return|if|else|for|in|of|try|catch|new|null|undefined|true|false|console|process|module|async)\b/g, '<span class="hl-keyword">$1</span>')
      // Types
      .replace(/\b(NexDb|Error|Promise|EventEmitter|Map|Set|Array|Object|String|Number|Boolean)\b/g, '<span class="hl-type">$1</span>')
      // Functions
      .replace(/\b(\w+)(?=\s*\()/g, '<span class="hl-fn">$1</span>');
  }

  function highlightJson(code) {
    return code
      // String values
      .replace(/(".*?")(\s*:\s*)(".*?")/g, '$1$2<span class="hl-string">$3</span>')
      // Number values
      .replace(/(".*?")(\s*:\s*)(\d+)/g, '$1$2<span class="hl-number">$3</span>')
      // Boolean/Null values
      .replace(/(".*?")(\s*:\s*)(true|false|null)/g, '$1$2<span class="hl-keyword">$3</span>')
      // String keys
      .replace(/(".*?")(\s*:)/g, '<span class="hl-keyword">$1</span>$2');
  }

  function highlightBash(code) {
    return code
      // Comments
      .replace(/(#.*)/g, '<span class="hl-comment">$1</span>')
      // Commands
      .replace(/\b(cargo|pip|npm|npx|nc|curl)\b/g, '<span class="hl-keyword">$1</span>')
      // Subcommands
      .replace(/\b(nexdb|install|add|run|serve|insert|get|update|delete|count|clean|completions|checkpoint|migrate|repl)\b/g, '<span class="hl-fn">$1</span>')
      // Strings
      .replace(/(".*?")/g, '<span class="hl-string">$1</span>')
      .replace(/('.*?')/g, '<span class="hl-string">$1</span>');
  }

  function highlightCode(element) {
    var rawText = element.textContent || element.innerText;
    var html = escapeHtml(rawText);

    if (element.classList.contains('language-rust')) {
      element.innerHTML = highlightRust(html);
    } else if (element.classList.contains('language-python')) {
      element.innerHTML = highlightPython(html);
    } else if (element.classList.contains('language-javascript') || element.classList.contains('language-js')) {
      element.innerHTML = highlightJavascript(html);
    } else if (element.classList.contains('language-json')) {
      element.innerHTML = highlightJson(element.innerHTML); // keeps nested layout
    } else if (element.classList.contains('language-bash') || element.classList.contains('language-sh')) {
      element.innerHTML = highlightBash(html);
    }
  }

  // Run highlighter
  document.querySelectorAll('code[class^="language-"]').forEach(highlightCode);


  // ─── 4. Sidebar Active Section Tracking ───
  var sidebarLinks = document.querySelectorAll('.docs-sidebar a');
  var sections = document.querySelectorAll('section[id]');

  if (sections.length && sidebarLinks.length) {
    var observer = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          var id = entry.target.id;
          sidebarLinks.forEach(function (link) {
            if (link.getAttribute('href') === '#' + id) {
              link.classList.add('active');
            } else {
              link.classList.remove('active');
            }
          });
        }
      });
    }, { rootMargin: '-10% 0px -70% 0px' });

    sections.forEach(function (s) {
      observer.observe(s);
    });
  }


  // ─── 5. Scroll Reveal Animation ───
  var animEls = document.querySelectorAll('.animate-in');
  if (animEls.length) {
    var animObserver = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          entry.target.classList.add('visible');
          animObserver.unobserve(entry.target);
        }
      });
    }, { threshold: 0.05 });
    
    animEls.forEach(function (el) {
      animObserver.observe(el);
    });
  }

});
