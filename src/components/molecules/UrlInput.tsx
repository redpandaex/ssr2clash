import { AlertCircle, CheckCircle2 } from 'lucide-react'
import { useState } from 'react'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

interface UrlInputProps {
  value: string
  onChange: (value: string) => void
  disabled?: boolean
}

export function UrlInput({ value, onChange, disabled }: UrlInputProps) {
  const [touched, setTouched] = useState(false)

  const isValidUrl = (url: string) => {
    if (!url) return null
    try {
      new URL(url)
      return true
    } catch {
      return false
    }
  }

  const validationState = touched ? isValidUrl(value) : null

  return (
    <div className="space-y-2">
      <Label htmlFor="subscription-url" className="text-sm font-medium">
        订阅链接
      </Label>
      <div className="relative">
        <Input
          id="subscription-url"
          type="url"
          placeholder="https://your-subscription-url.com"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onBlur={() => setTouched(true)}
          disabled={disabled}
          className={
            validationState === false
              ? 'border-red-500 focus-visible:ring-red-500'
              : validationState === true
                ? 'border-green-500 focus-visible:ring-green-500'
                : ''
          }
        />
        {validationState !== null && (
          <div className="absolute right-3 top-1/2 -translate-y-1/2">
            {validationState ? (
              <CheckCircle2 className="h-4 w-4 text-green-500" />
            ) : (
              <AlertCircle className="h-4 w-4 text-red-500" />
            )}
          </div>
        )}
      </div>
      {validationState === false && (
        <p className="text-xs text-red-500">请输入有效的 URL</p>
      )}
    </div>
  )
}