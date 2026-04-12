/**
 * XSS 防护工具
 * 用于对 AI 生成的内容进行安全过滤
 */

// DOMPurify 配置 - 需要安装: npm install dompurify
// import DOMPurify from 'dompurify'

/**
 * 安全的 HTML 过滤
 * 只允许基本的文本格式化标签
 */
export function sanitizeHtml(dirty: string): string {
  // 如果 DOMPurify 可用，使用它
  // if (typeof DOMPurify !== 'undefined') {
  //   return DOMPurify.sanitize(dirty, {
  //     ALLOWED_TAGS: ['b', 'i', 'em', 'strong', 'p', 'br', 'ul', 'ol', 'li', 'a', 'span'],
  //     ALLOWED_ATTR: ['href', 'class', 'style'],
  //   })
  // }

  // 基础实现：去除所有 HTML 标签（保守方案）
  const temp = document.createElement('div')
  temp.textContent = dirty
  return temp.innerHTML
}

/**
 * 纯文本过滤
 * 完全去除所有 HTML
 */
export function sanitizeText(text: string): string {
  const temp = document.createElement('div')
  temp.textContent = text
  return temp.textContent || ''
}

/**
 * 链接过滤
 * 只允许安全的链接协议
 */
export function sanitizeUrl(url: string): string {
  try {
    const parsed = new URL(url)
    // 只允许 http/https 协议
    if (!['http:', 'https:'].includes(parsed.protocol)) {
      return '#'
    }
    return url
  } catch {
    return '#'
  }
}

/**
 * 关键字过滤
 * 过滤敏感词和不当内容
 */
export function filterKeywords(text: string): string {
  const sensitiveWords: string[] = []

  let filtered = text
  for (const word of sensitiveWords) {
    const regex = new RegExp(word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi')
    filtered = filtered.replace(regex, '***')
  }

  return filtered
}
