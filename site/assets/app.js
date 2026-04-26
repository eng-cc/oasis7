(() => {
  document.documentElement.setAttribute("data-js", "true");
  const LANGUAGE_REDIRECT_KEY = "oasis7_pages_lang_redirect_done_v1";
  const LANGUAGE_MANUAL_CHOICE_KEY = "oasis7_pages_lang_manual_choice_v1";

  const safeGetStorage = (key) => {
    try {
      return window.localStorage.getItem(key);
    } catch {
      return null;
    }
  };

  const safeSetStorage = (key, value) => {
    try {
      window.localStorage.setItem(key, value);
    } catch {
      return;
    }
  };

  const resolvePreferredLanguage = () => {
    const browserLanguages = Array.isArray(window.navigator.languages)
      ? window.navigator.languages
      : [];
    const firstLanguage = browserLanguages.find(
      (lang) => typeof lang === "string" && lang.trim().length > 0,
    );
    return String(firstLanguage || window.navigator.language || "").toLowerCase();
  };

  const isChineseEntryPath = (pathname) => {
    const onEnglishPage = /\/en\/(?:index\.html)?$/.test(pathname);
    if (onEnglishPage) {
      return false;
    }
    return /\/(?:index\.html)?$/.test(pathname);
  };

  const toEnglishEntryPath = (pathname) => {
    if (pathname.endsWith("/index.html")) {
      return `${pathname.slice(0, -"index.html".length)}en/`;
    }
    if (pathname.endsWith("/")) {
      return `${pathname}en/`;
    }
    return `${pathname}/en/`;
  };

  const isDocsPath = (pathname) => /\/doc\/(?:cn|en)\//.test(pathname);

  const maybeRedirectByLanguageOnFirstVisit = () => {
    const manualChoice = safeGetStorage(LANGUAGE_MANUAL_CHOICE_KEY);
    if (manualChoice === "zh" || manualChoice === "en") {
      return;
    }

    if (safeGetStorage(LANGUAGE_REDIRECT_KEY) === "1") {
      return;
    }

    const { pathname, search, hash } = window.location;
    if (isDocsPath(pathname)) {
      return;
    }

    const preferredLanguage = resolvePreferredLanguage();
    const prefersEnglish = preferredLanguage.startsWith("en");

    // Entry-page visits are checked once; docs paths are intentionally excluded.
    safeSetStorage(LANGUAGE_REDIRECT_KEY, "1");

    if (!prefersEnglish) {
      return;
    }

    if (!isChineseEntryPath(pathname)) {
      return;
    }

    const targetPath = toEnglishEntryPath(pathname);
    window.location.replace(`${targetPath}${search}${hash}`);
  };

  const bindLanguageChoicePersistence = () => {
    document.querySelectorAll("[data-lang-choice]").forEach((link) => {
      link.addEventListener("click", () => {
        const choice = link.getAttribute("data-lang-choice");
        if (choice === "zh" || choice === "en") {
          safeSetStorage(LANGUAGE_MANUAL_CHOICE_KEY, choice);
          safeSetStorage(LANGUAGE_REDIRECT_KEY, "1");
        }
      });
    });
  };

  const bindSectionReveal = () => {
    const revealNodes = Array.from(document.querySelectorAll("[data-reveal]"));
    if (!revealNodes.length) {
      return;
    }

    const revealAll = () => {
      revealNodes.forEach((node) => node.classList.add("revealed"));
    };

    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      revealAll();
      return;
    }

    const observer = new IntersectionObserver(
      (entries, obs) => {
        entries.forEach((entry) => {
          if (!entry.isIntersecting) {
            return;
          }
          entry.target.classList.add("revealed");
          obs.unobserve(entry.target);
        });
      },
      {
        threshold: 0.18,
      },
    );

    revealNodes.forEach((node) => observer.observe(node));

    window.setTimeout(() => {
      revealAll();
      observer.disconnect();
    }, 1800);
  };

  const bindCounters = () => {
    const counters = Array.from(document.querySelectorAll("[data-counter-target]"));
    if (!counters.length) {
      return;
    }

    const prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    const renderFinal = (node) => {
      const target = Number(node.getAttribute("data-counter-target") || "0");
      node.textContent = String(Math.max(0, Math.round(target)));
    };

    if (prefersReducedMotion) {
      counters.forEach(renderFinal);
      return;
    }

    const animate = (node) => {
      const target = Number(node.getAttribute("data-counter-target") || "0");
      const safeTarget = Math.max(0, Math.round(target));
      const startedAt = performance.now();
      const duration = Math.min(1400, 520 + safeTarget * 20);

      const tick = (now) => {
        const progress = Math.min(1, (now - startedAt) / duration);
        const eased = 1 - Math.pow(1 - progress, 3);
        node.textContent = String(Math.round(safeTarget * eased));
        if (progress < 1) {
          window.requestAnimationFrame(tick);
        }
      };

      window.requestAnimationFrame(tick);
    };

    const observer = new IntersectionObserver(
      (entries, obs) => {
        entries.forEach((entry) => {
          if (!entry.isIntersecting) {
            return;
          }
          animate(entry.target);
          obs.unobserve(entry.target);
        });
      },
      {
        threshold: 0.4,
      },
    );

    counters.forEach((node) => observer.observe(node));

    window.setTimeout(() => {
      counters.forEach((node) => {
        if (node.textContent === "0") {
          const target = Number(node.getAttribute("data-counter-target") || "0");
          node.textContent = String(Math.max(0, Math.round(target)));
        }
      });
      observer.disconnect();
    }, 2200);
  };

  const bindHeroCanvas = () => {
    const canvas = document.querySelector("[data-hero-canvas]");
    if (!(canvas instanceof HTMLCanvasElement)) {
      return;
    }

    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    if (reducedMotion.matches) {
      return;
    }

    const context = canvas.getContext("2d");
    if (!context) {
      return;
    }

    let width = 0;
    let height = 0;
    let rafId = 0;
    let lastFrameAt = 0;
    let sweepX = 0;
    let pointerX = -1;
    let pointerY = -1;
    let pointerActive = false;
    let pointerEnergy = 0;

    /** @type {Array<{x:number,y:number,vx:number,vy:number,size:number}>} */
    let nodes = [];

    const randomIn = (min, max) => min + Math.random() * (max - min);

    const nodeCountForViewport = () => {
      const area = width * height;
      if (area < 220000) {
        return 12;
      }
      if (area < 500000) {
        return 18;
      }
      return 26;
    };

    const rebuildNodes = () => {
      const count = nodeCountForViewport();
      nodes = Array.from({ length: count }, () => ({
        x: randomIn(0, width),
        y: randomIn(0, height),
        vx: randomIn(-0.16, 0.16),
        vy: randomIn(-0.14, 0.14),
        size: randomIn(1.2, 2.4),
      }));
    };

    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      const nextWidth = Math.max(1, Math.floor(rect.width));
      const nextHeight = Math.max(1, Math.floor(rect.height));
      if (nextWidth === width && nextHeight === height) {
        return;
      }

      width = nextWidth;
      height = nextHeight;
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      canvas.width = Math.floor(width * dpr);
      canvas.height = Math.floor(height * dpr);
      context.setTransform(dpr, 0, 0, dpr, 0, 0);
      rebuildNodes();
    };

    const setPointer = (clientX, clientY, boost = 1) => {
      const rect = canvas.getBoundingClientRect();
      pointerX = Math.max(0, Math.min(width, clientX - rect.left));
      pointerY = Math.max(0, Math.min(height, clientY - rect.top));
      pointerActive = true;
      pointerEnergy = Math.min(1, Math.max(pointerEnergy, boost));
    };

    const fadePointer = () => {
      pointerActive = false;
    };

    const stepNodes = () => {
      const interactionRadius = width < 680 ? 105 : 148;
      const interactionRadiusSquare = interactionRadius * interactionRadius;
      const interactionScale = 0.075 * (0.38 + pointerEnergy * 0.62);

      for (const node of nodes) {
        if (pointerEnergy > 0 && pointerX >= 0 && pointerY >= 0) {
          const dx = node.x - pointerX;
          const dy = node.y - pointerY;
          const distanceSquare = dx * dx + dy * dy;
          if (distanceSquare < interactionRadiusSquare) {
            const distance = Math.max(1, Math.sqrt(distanceSquare));
            const pull = Math.pow(1 - distance / interactionRadius, 1.9) * interactionScale;
            node.vx += (dx / distance) * pull;
            node.vy += (dy / distance) * pull;
          }
        }

        node.x += node.vx;
        node.y += node.vy;

        node.vx *= 0.994;
        node.vy *= 0.994;

        if (node.x <= 0 || node.x >= width) {
          node.vx *= -1;
          node.x = Math.max(0, Math.min(width, node.x));
        }
        if (node.y <= 0 || node.y >= height) {
          node.vy *= -1;
          node.y = Math.max(0, Math.min(height, node.y));
        }
      }
    };

    const drawGridScan = () => {
      const sweepSpeed = 0.82 * (1 + pointerEnergy * 1.7);
      sweepX = (sweepX + sweepSpeed) % (width + 180);
      const gradient = context.createLinearGradient(sweepX - 170, 0, sweepX + 40, 0);
      gradient.addColorStop(0, "rgba(68, 231, 197, 0)");
      gradient.addColorStop(0.55, `rgba(68, 231, 197, ${0.1 + pointerEnergy * 0.08})`);
      gradient.addColorStop(1, "rgba(68, 231, 197, 0)");
      context.fillStyle = gradient;
      context.fillRect(0, 0, width, height);

      if (pointerEnergy > 0.02 && pointerX >= 0 && pointerY >= 0) {
        const halo = context.createRadialGradient(
          pointerX,
          pointerY,
          0,
          pointerX,
          pointerY,
          125 + pointerEnergy * 35,
        );
        halo.addColorStop(0, `rgba(124, 241, 139, ${0.19 * pointerEnergy})`);
        halo.addColorStop(1, "rgba(124, 241, 139, 0)");
        context.fillStyle = halo;
        context.fillRect(0, 0, width, height);
      }
    };

    const drawLinksAndNodes = () => {
      const maxDistance = width < 680 ? 115 : 150;
      const maxDistanceSquare = maxDistance * maxDistance;

      for (let i = 0; i < nodes.length; i += 1) {
        const first = nodes[i];
        for (let j = i + 1; j < nodes.length; j += 1) {
          const second = nodes[j];
          const dx = first.x - second.x;
          const dy = first.y - second.y;
          const distanceSquare = dx * dx + dy * dy;
          if (distanceSquare > maxDistanceSquare) {
            continue;
          }
          const distance = Math.sqrt(distanceSquare);
          const alphaBoost = pointerEnergy > 0 ? pointerEnergy * 0.1 : 0;
          const alpha = Math.pow(1 - distance / maxDistance, 1.8) * (0.24 + alphaBoost);
          context.strokeStyle = `rgba(110, 213, 235, ${alpha})`;
          context.lineWidth = 1;
          context.beginPath();
          context.moveTo(first.x, first.y);
          context.lineTo(second.x, second.y);
          context.stroke();
        }
      }

      for (const node of nodes) {
        let nodeAlpha = 0.72;
        if (pointerEnergy > 0 && pointerX >= 0 && pointerY >= 0) {
          const dx = node.x - pointerX;
          const dy = node.y - pointerY;
          const dist = Math.sqrt(dx * dx + dy * dy);
          if (dist < 130) {
            nodeAlpha = 0.72 + (1 - dist / 130) * 0.24 * pointerEnergy;
          }
        }
        context.fillStyle = `rgba(132, 255, 188, ${Math.min(1, nodeAlpha)})`;
        context.beginPath();
        context.arc(node.x, node.y, node.size, 0, Math.PI * 2);
        context.fill();
      }
    };

    const paint = (now) => {
      if (document.hidden) {
        rafId = 0;
        return;
      }

      if (now - lastFrameAt < 32) {
        rafId = window.requestAnimationFrame(paint);
        return;
      }
      lastFrameAt = now;

      if (!pointerActive) {
        pointerEnergy *= 0.94;
        if (pointerEnergy < 0.01) {
          pointerEnergy = 0;
        }
      }

      context.clearRect(0, 0, width, height);
      drawGridScan();
      stepNodes();
      drawLinksAndNodes();
      rafId = window.requestAnimationFrame(paint);
    };

    resize();
    window.addEventListener("resize", resize, { passive: true });

    const heroSection = canvas.closest(".section-hero");
    if (heroSection) {
      heroSection.addEventListener(
        "pointermove",
        (event) => {
          setPointer(event.clientX, event.clientY, event.pointerType === "touch" ? 0.75 : 1);
        },
        { passive: true },
      );
      heroSection.addEventListener(
        "pointerdown",
        (event) => {
          setPointer(event.clientX, event.clientY, 1);
        },
        { passive: true },
      );
      heroSection.addEventListener("pointerleave", fadePointer, { passive: true });
      heroSection.addEventListener("pointercancel", fadePointer, { passive: true });
      heroSection.addEventListener(
        "pointerup",
        (event) => {
          if (event.pointerType === "touch") {
            fadePointer();
          }
        },
        { passive: true },
      );
    }

    document.addEventListener("visibilitychange", () => {
      if (document.hidden) {
        if (rafId) {
          window.cancelAnimationFrame(rafId);
          rafId = 0;
        }
        pointerActive = false;
        return;
      }
      if (!rafId) {
        lastFrameAt = 0;
        rafId = window.requestAnimationFrame(paint);
      }
    });

    if (typeof reducedMotion.addEventListener === "function") {
      reducedMotion.addEventListener("change", (event) => {
        if (!event.matches) {
          if (!rafId) {
            lastFrameAt = 0;
            rafId = window.requestAnimationFrame(paint);
          }
          return;
        }
        if (rafId) {
          window.cancelAnimationFrame(rafId);
          rafId = 0;
        }
        context.clearRect(0, 0, width, height);
      });
    }

    rafId = window.requestAnimationFrame(paint);
  };

  const bindActiveNav = () => {
    const nav = document.querySelector("[data-section-nav]");
    if (!nav) {
      return;
    }

    const links = Array.from(nav.querySelectorAll("a[href^='#']"));
    if (!links.length) {
      return;
    }

    const linkMap = new Map();
    const sections = [];

    links.forEach((link) => {
      const id = link.getAttribute("href")?.slice(1);
      if (!id) {
        return;
      }
      const section = document.getElementById(id);
      if (!section) {
        return;
      }
      linkMap.set(section, link);
      sections.push(section);
    });

    const setActiveLink = (link) => {
      links.forEach((node) => {
        if (node === link) {
          node.classList.add("active");
        } else {
          node.classList.remove("active");
        }
      });
    };

    if (links[0]) {
      setActiveLink(links[0]);
    }

    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((entry) => entry.isIntersecting)
          .sort((first, second) => second.intersectionRatio - first.intersectionRatio);
        if (!visible.length) {
          return;
        }
        const current = visible[0].target;
        const link = linkMap.get(current);
        if (link) {
          setActiveLink(link);
        }
      },
      {
        rootMargin: "-28% 0px -54% 0px",
        threshold: [0.2, 0.35, 0.5],
      },
    );

    sections.forEach((section) => observer.observe(section));
  };

  const bindTimelineFilters = () => {
    const controls = document.querySelector("[data-timeline-controls]");
    const timeline = document.querySelector("[data-timeline-group]");
    if (!controls || !timeline) {
      return;
    }

    const buttons = Array.from(controls.querySelectorAll("[data-timeline-filter]"));
    const items = Array.from(timeline.querySelectorAll("[data-timeline-state]"));
    if (!buttons.length || !items.length) {
      return;
    }

    const applyFilter = (filter) => {
      items.forEach((item) => {
        const state = item.getAttribute("data-timeline-state") || "";
        const visible = filter === "all" || state === filter;
        item.setAttribute("data-hidden", visible ? "false" : "true");
      });

      buttons.forEach((button) => {
        const isActive = button.getAttribute("data-timeline-filter") === filter;
        button.classList.toggle("is-active", isActive);
        button.setAttribute("aria-pressed", isActive ? "true" : "false");
      });
    };

    buttons.forEach((button) => {
      button.addEventListener("click", () => {
        const filter = button.getAttribute("data-timeline-filter") || "all";
        applyFilter(filter);
      });
    });

    applyFilter("all");
  };

  const bindStoryPathHighlight = () => {
    const steps = Array.from(document.querySelectorAll("[data-story-step]"));
    if (!steps.length) {
      return;
    }

    const setActiveStep = (active) => {
      steps.forEach((step) => {
        step.classList.toggle("is-active", step === active);
      });
    };

    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      if (steps[0]) {
        setActiveStep(steps[0]);
      }
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((entry) => entry.isIntersecting)
          .sort((first, second) => second.intersectionRatio - first.intersectionRatio);
        if (!visible.length) {
          return;
        }
        setActiveStep(visible[0].target);
      },
      {
        threshold: [0.35, 0.5, 0.7],
        rootMargin: "-10% 0px -18% 0px",
      },
    );

    steps.forEach((step) => observer.observe(step));

    if (steps[0]) {
      setActiveStep(steps[0]);
    }
  };

  const bindProofSwitcher = () => {
    const controls = document.querySelector("[data-proof-controls]");
    if (!controls) {
      return;
    }

    const buttons = Array.from(controls.querySelectorAll("[data-proof-tab]"));
    const panels = Array.from(document.querySelectorAll("[data-proof-code][data-proof-panel]"));
    const events = Array.from(document.querySelectorAll("[data-proof-event]"));

    if (!buttons.length || !panels.length || !events.length) {
      return;
    }

    const applyTab = (tab) => {
      buttons.forEach((button) => {
        const isActive = button.getAttribute("data-proof-tab") === tab;
        button.classList.toggle("is-active", isActive);
        button.setAttribute("aria-pressed", isActive ? "true" : "false");
      });

      panels.forEach((panel) => {
        const visible = panel.getAttribute("data-proof-panel") === tab;
        panel.setAttribute("data-proof-visible", visible ? "true" : "false");
      });

      events.forEach((item) => {
        const visible = item.getAttribute("data-proof-event") === tab;
        item.setAttribute("data-proof-visible", visible ? "true" : "false");
      });
    };

    buttons.forEach((button, index) => {
      button.addEventListener("click", () => {
        const tab = button.getAttribute("data-proof-tab") || "minimal";
        applyTab(tab);
      });

      button.addEventListener("keydown", (event) => {
        if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") {
          return;
        }

        event.preventDefault();
        const delta = event.key === "ArrowRight" ? 1 : -1;
        const nextIndex = (index + delta + buttons.length) % buttons.length;
        const nextButton = buttons[nextIndex];
        nextButton.focus();
        const tab = nextButton.getAttribute("data-proof-tab") || "minimal";
        applyTab(tab);
      });
    });

    applyTab("minimal");
  };

  const bindLatestReleaseMeta = () => {
    const tagNodes = Array.from(document.querySelectorAll("[data-release-tag]"));
    const dateNodes = Array.from(document.querySelectorAll("[data-release-date]"));
    const notesLinks = Array.from(document.querySelectorAll("[data-release-notes-link]"));
    if (!tagNodes.length && !dateNodes.length && !notesLinks.length) {
      return;
    }
    if (typeof window.fetch !== "function") {
      return;
    }

    const apiUrl = "https://api.github.com/repos/eng-cc/oasis7/releases/latest";
    const controller = typeof AbortController === "function" ? new AbortController() : null;
    let timeoutId = 0;
    if (controller) {
      timeoutId = window.setTimeout(() => {
        controller.abort();
      }, 4500);
    }

    const requestOptions = {
      headers: {
        Accept: "application/vnd.github+json",
      },
    };
    if (controller) {
      requestOptions.signal = controller.signal;
    }

    window
      .fetch(apiUrl, requestOptions)
      .then((response) => {
        if (!response.ok) {
          throw new Error(`release lookup failed: ${response.status}`);
        }
        return response.json();
      })
      .then((release) => {
        const tagName =
          typeof release.tag_name === "string" && release.tag_name.trim().length > 0
            ? release.tag_name.trim()
            : "latest";
        tagNodes.forEach((node) => {
          node.textContent = tagName;
        });

        const releaseUrl =
          typeof release.html_url === "string" && release.html_url.trim().length > 0
            ? release.html_url.trim()
            : "https://github.com/eng-cc/oasis7/releases/latest";
        notesLinks.forEach((node) => {
          node.setAttribute("href", releaseUrl);
        });

        const publishedAt = Date.parse(String(release.published_at || ""));
        if (!Number.isFinite(publishedAt)) {
          return;
        }

        const pageLang = String(document.documentElement.lang || "").toLowerCase();
        const locale = pageLang.startsWith("zh") ? "zh-CN" : "en-US";
        const formattedDate = new Intl.DateTimeFormat(locale, {
          year: "numeric",
          month: "short",
          day: "2-digit",
        }).format(new Date(publishedAt));

        dateNodes.forEach((node) => {
          const prefix = String(node.getAttribute("data-release-date-prefix") || "").trim();
          node.textContent = prefix ? `${prefix}: ${formattedDate}` : formattedDate;
        });
      })
      .catch(() => {
        // Keep static fallback text when request fails.
      })
      .finally(() => {
        if (timeoutId) {
          window.clearTimeout(timeoutId);
        }
      });
  };

  const bindReleaseDownloadSurface = () => {
    const surfaces = Array.from(document.querySelectorAll("[data-download-surface]"));
    if (!surfaces.length) {
      return;
    }

    const pageLang = String(document.documentElement.lang || "").toLowerCase();
    const isZh = pageLang.startsWith("zh");
    const platformLabels = {
      windows: "Windows x64",
      macos: "macOS x64",
      linux: "Linux x64",
    };
    const buildRecommendationText = (platformId, mode) => {
      const platformLabel = platformLabels[platformId] || platformId;
      if (isZh) {
        if (mode === "auto") {
          return {
            badge: "当前设备推荐",
            footnote: `已按当前设备优先选中 ${platformLabel}；如果你要给其他机器下载，可切换上方平台按钮。`,
          };
        }
        if (mode === "manual") {
          return {
            badge: "手动切换平台",
            footnote: `当前显示的是你手动切换后的 ${platformLabel} 主包；如果要恢复自动判断，请刷新页面后按当前设备重新选择。`,
          };
        }
        return {
          badge: "按平台选择主包",
          footnote: `当前无法可靠识别设备平台，默认展示 ${platformLabel} 主包；如果你要给其他机器下载，可切换上方平台按钮。`,
        };
      }

      if (mode === "auto") {
        return {
          badge: "Recommended for this device",
          footnote: `The page auto-selected ${platformLabel} for the current device. Switch platforms above if you are downloading for a different machine.`,
        };
      }
      if (mode === "manual") {
        return {
          badge: "Manually selected platform",
          footnote: `You manually switched to the ${platformLabel} primary package. Refresh the page if you want the surface to follow device detection again.`,
        };
      }
      return {
        badge: "Choose a platform",
        footnote: `The page could not confidently detect the current device platform, so it is showing the ${platformLabel} primary package by default. Switch platforms above if needed.`,
      };
    };

    const detectPreferredPlatform = () => {
      const uaData = window.navigator && window.navigator.userAgentData ? window.navigator.userAgentData : null;
      const userAgent = window.navigator ? String(window.navigator.userAgent || "").toLowerCase() : "";
      const platform = window.navigator ? String(window.navigator.platform || "").toLowerCase() : "";
      const uaPlatform = uaData ? String(uaData.platform || "").toLowerCase() : "";
      const candidateText = [uaPlatform, platform, userAgent]
        .filter(Boolean)
        .join(" ");
      const isMobile =
        (uaData && uaData.mobile === true) ||
        /android|iphone|ipad|ipod|mobile/.test(candidateText);
      if (isMobile) {
        return "";
      }

      const candidates = [
        uaPlatform,
        platform,
        userAgent,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();

      if (candidates.includes("mac") || candidates.includes("darwin")) {
        return "macos";
      }
      if (/\bwindows\b|\bwin32\b|\bwin64\b|\bwinnt\b/.test(candidates)) {
        return "windows";
      }
      if (candidates.includes("linux") || candidates.includes("x11")) {
        return "linux";
      }
      return "";
    };

    surfaces.forEach((surface) => {
      const buttons = Array.from(surface.querySelectorAll("[data-download-platform-button]"));
      const sourceNodes = Array.from(surface.querySelectorAll("[data-download-platform-source]"));
      if (!buttons.length || !sourceNodes.length) {
        return;
      }

      const sourceMap = new Map();
      sourceNodes.forEach((node) => {
        const platformId = String(node.getAttribute("data-download-platform-source") || "").trim();
        if (!platformId) {
          return;
        }

        const readText = (selector) => {
          const target = node.querySelector(selector);
          return target ? target.textContent.trim() : "";
        };

        sourceMap.set(platformId, {
          url: String(node.getAttribute("data-download-source-url") || "").trim(),
          title: readText("[data-download-source-title]"),
          copy: readText("[data-download-source-copy]"),
          linkLabel: readText("[data-download-source-link-label]"),
          requirements: readText("[data-download-source-requirements]"),
          install: readText("[data-download-source-install]"),
          trust: readText("[data-download-source-trust]"),
          support: readText("[data-download-source-support]"),
        });
      });

      const badgeNode = surface.querySelector("[data-download-primary-badge]");
      const titleNode = surface.querySelector("[data-download-primary-title]");
      const copyNode = surface.querySelector("[data-download-primary-copy]");
      const linkNode = surface.querySelector("[data-download-primary-link]");
      const requirementsNode = surface.querySelector("[data-download-primary-requirements]");
      const installNode = surface.querySelector("[data-download-primary-install]");
      const trustNode = surface.querySelector("[data-download-primary-trust]");
      const supportNode = surface.querySelector("[data-download-primary-support]");
      const footnoteNode = surface.querySelector("[data-download-primary-footnote]");

      const applyPlatform = (platformId, selectionMode) => {
        const next = sourceMap.get(platformId);
        if (!next) {
          return;
        }

        surface.setAttribute("data-download-active-platform", platformId);
        buttons.forEach((button) => {
          const isActive = String(button.getAttribute("data-download-platform-button") || "") === platformId;
          button.classList.toggle("is-active", isActive);
          button.setAttribute("aria-pressed", isActive ? "true" : "false");
        });

        const recommendationText = buildRecommendationText(platformId, selectionMode);
        if (badgeNode) {
          badgeNode.textContent = recommendationText.badge;
        }
        if (titleNode && next.title) {
          titleNode.textContent = next.title;
        }
        if (copyNode && next.copy) {
          copyNode.textContent = next.copy;
        }
        if (linkNode && next.url) {
          linkNode.setAttribute("href", next.url);
        }
        if (linkNode && next.linkLabel) {
          linkNode.textContent = next.linkLabel;
        }
        if (requirementsNode && next.requirements) {
          requirementsNode.textContent = next.requirements;
        }
        if (installNode && next.install) {
          installNode.textContent = next.install;
        }
        if (trustNode && next.trust) {
          trustNode.textContent = next.trust;
        }
        if (supportNode && next.support) {
          supportNode.textContent = next.support;
        }
        if (footnoteNode) {
          footnoteNode.textContent = recommendationText.footnote;
        }
      };

      buttons.forEach((button) => {
        button.addEventListener("click", () => {
          applyPlatform(String(button.getAttribute("data-download-platform-button") || ""), "manual");
        });
      });

      const detected = detectPreferredPlatform();
      const fallback =
        buttons.length > 0
          ? String(buttons[0].getAttribute("data-download-platform-button") || "")
          : "";
      const initialPlatform = sourceMap.has(detected) ? detected : fallback;
      const initialMode = detected && sourceMap.has(detected) ? "auto" : "neutral";
      applyPlatform(initialPlatform, initialMode);
    });
  };

  maybeRedirectByLanguageOnFirstVisit();
  bindLanguageChoicePersistence();
  bindReleaseDownloadSurface();

  const menu = document.querySelector("[data-menu]");
  const toggle = document.querySelector("[data-menu-toggle]");
  const langToggle = document.querySelector("[data-lang-toggle]");
  const langPopover = document.querySelector("[data-lang-popover]");
  const langItems = Array.from(document.querySelectorAll("[data-lang-item]"));
  const yearNode = document.querySelector("[data-year]");

  if (yearNode) {
    yearNode.textContent = String(new Date().getFullYear());
  }

  if (menu && toggle) {
    const menuId = menu.id || "site-nav-menu";
    if (!menu.id) {
      menu.id = menuId;
    }
    if (!toggle.hasAttribute("aria-controls")) {
      toggle.setAttribute("aria-controls", menu.id);
    }

    const setMenuOpen = (open) => {
      menu.setAttribute("data-open", open ? "true" : "false");
      toggle.setAttribute("aria-expanded", open ? "true" : "false");
    };

    setMenuOpen(menu.getAttribute("data-open") === "true");

    toggle.addEventListener("click", () => {
      const opened = menu.getAttribute("data-open") === "true";
      setMenuOpen(!opened);
    });

    menu.querySelectorAll("a").forEach((link) => {
      link.addEventListener("click", () => {
        setMenuOpen(false);
      });
    });

    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        setMenuOpen(false);
      }
    });
  }

  if (langToggle && langPopover) {
    const closePopover = () => {
      langPopover.setAttribute("data-open", "false");
      langToggle.setAttribute("aria-expanded", "false");
    };

    const openPopover = () => {
      langPopover.setAttribute("data-open", "true");
      langToggle.setAttribute("aria-expanded", "true");
    };

    langToggle.addEventListener("click", () => {
      const opened = langPopover.getAttribute("data-open") === "true";
      if (opened) {
        closePopover();
      } else {
        openPopover();
      }
    });

    langToggle.addEventListener("keydown", (event) => {
      if (event.key === "ArrowDown" || event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openPopover();
        if (langItems[0]) {
          langItems[0].focus();
        }
      }
    });

    langItems.forEach((item, index) => {
      item.addEventListener("keydown", (event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          closePopover();
          langToggle.focus();
          return;
        }

        if (event.key === "ArrowDown") {
          event.preventDefault();
          const next = langItems[index + 1] || langItems[0];
          next.focus();
          return;
        }

        if (event.key === "ArrowUp") {
          event.preventDefault();
          const previous = langItems[index - 1] || langItems[langItems.length - 1];
          previous.focus();
        }
      });
    });

    document.addEventListener("click", (event) => {
      const target = event.target;
      if (!(target instanceof Node)) {
        return;
      }
      if (langToggle.contains(target) || langPopover.contains(target)) {
        return;
      }
      closePopover();
    });

    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        closePopover();
      }
    });

    langPopover.querySelectorAll("a").forEach((link) => {
      link.addEventListener("click", () => {
        closePopover();
      });
    });
  }

  bindSectionReveal();
  bindHeroCanvas();
  bindCounters();
  bindActiveNav();
  bindTimelineFilters();
  bindStoryPathHighlight();
  bindProofSwitcher();
  bindLatestReleaseMeta();
})();
