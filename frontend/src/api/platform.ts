import http from './http'

export interface PlatformConfig {
  platform: string
  name: string
  enabled: boolean
  status: 'connected' | 'disconnected' | 'expired'
  last_login?: string
}

export interface QrCodeInfo {
  qr_id: string
  qr_url: string
  expire_at: string
}

export const platformApi = {
  list() {
    return http.get<{ platforms: PlatformConfig[] }>('/platforms')
  },
  update(platform: string, data: { enabled: boolean }) {
    return http.put<{ platform: PlatformConfig }>(`/platforms/${platform}`, data)
  },
  generateQr(platform: string) {
    return http.post<{ qr_info: QrCodeInfo }>('/qr/generate', { platform })
  },
  queryQrStatus(platform: string, qrId: string) {
    return http.get<{ status: string }>(`/qr/status/${platform}/${qrId}`)
  },
  confirmQrLogin(qrId: string) {
    return http.post<{ result: string }>('/qr/confirm', { qr_id: qrId })
  },
  waitQrConfirmation(platform: string, qrId: string) {
    return http.get<{ result: string }>(`/qr/wait/${platform}/${qrId}`)
  },
}
