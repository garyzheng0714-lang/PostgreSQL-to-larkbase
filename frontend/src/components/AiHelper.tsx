import { useState } from "react";
import { Sparkles, ChevronDown, Copy, Check } from "lucide-react";
import { AI_FILL_PROMPT } from "../lib/prompt";

/**
 * 「让 AI 帮你填」——本地 agent 集成入口。
 * 归一设计：复制一段提示词发给用户自己的 AI，AI 回一行 postgresql:// URI，
 * 粘回上方同一个连接串框即可。不新增独立粘贴框（URI 框本身就是粘贴目标）。
 */
export function AiHelper() {
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);

  const copyPrompt = async () => {
    try {
      await navigator.clipboard.writeText(AI_FILL_PROMPT);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // 剪贴板不可用时降级：选中文本由用户手动复制
      const ta = document.getElementById(
        "db-ai-prompt-fallback"
      ) as HTMLTextAreaElement | null;
      if (ta) {
        ta.style.display = "block";
        ta.focus();
        ta.select();
      }
    }
  };

  return (
    <div className="db-ai">
      <button
        type="button"
        className="db-ai__trigger"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <Sparkles className="db-ai__glyph" />
        不知道连接信息？让 AI 帮你填
        <ChevronDown className="db-ai__chev" />
      </button>

      <div className={`db-ai__wrap${open ? " is-open" : ""}`}>
        <div className="db-ai__panel">
          <div className="db-ai__panel-inner">
            <ol className="db-ai__steps">
              <li>把下面这段提示词复制，发给你常用的 AI（如 Claude、ChatGPT）。</li>
              <li>AI 会回你一行连接串，粘贴到上方「连接串」框里。</li>
              <li>点「连接」即可。建议用只读账号，更安全。</li>
            </ol>
            <button
              type="button"
              className={`db-copy${copied ? " db-copy--done" : ""}`}
              onClick={copyPrompt}
            >
              {copied ? <Check /> : <Copy />}
              {copied ? "已复制，去发给 AI" : "复制提示词"}
            </button>
            <p className="db-hint" style={{ marginTop: 10 }}>
              建议用只读账号；不放心可让 AI 用占位符代替密码，连接前自己补上。
            </p>
            <textarea
              id="db-ai-prompt-fallback"
              readOnly
              value={AI_FILL_PROMPT}
              style={{ display: "none", width: "100%", marginTop: 10, height: 120 }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
