import React, { useState, useEffect } from 'react';
import { startProxyV2, stopProxyV2, getProxyV2Status } from '../api';
import { listen } from '@tauri-apps/api/event';

interface ProxyEvent {
  timestamp: string;
  type: 'request' | 'response' | 'websocket';
  data: string;
}

export const ProxyV2Control: React.FC = () => {
  const [isRunning, setIsRunning] = useState(false);
  const [port, setPort] = useState(8100);
  const [isLoading, setIsLoading] = useState(false);
  const [events, setEvents] = useState<ProxyEvent[]>([]);

  useEffect(() => {
    checkStatus();

    // 프록시 이벤트 리스너 설정
    const unlistenRequest = listen('proxy_request', (event) => {
      addEvent('request', event.payload as string);
    });

    const unlistenResponse = listen('proxy_response', (event) => {
      addEvent('response', event.payload as string);
    });

    const unlistenWebSocket = listen('proxy_websocket', (event) => {
      addEvent('websocket', event.payload as string);
    });

    return () => {
      unlistenRequest.then((f) => f());
      unlistenResponse.then((f) => f());
      unlistenWebSocket.then((f) => f());
    };
  }, []);

  const addEvent = (type: 'request' | 'response' | 'websocket', data: string) => {
    const newEvent: ProxyEvent = {
      timestamp: new Date().toLocaleTimeString(),
      type,
      data: data.length > 200 ? data.substring(0, 200) + '...' : data,
    };

    setEvents((prev) => [newEvent, ...prev.slice(0, 49)]); // 최대 50개 이벤트 유지
  };

  const checkStatus = async () => {
    try {
      const status = await getProxyV2Status();
      setIsRunning(status);
    } catch (error) {
      console.error('프록시 상태 확인 실패:', error);
    }
  };

  const handleStart = async () => {
    setIsLoading(true);
    try {
      await startProxyV2(port);
      setIsRunning(true);
      setEvents([]); // 이벤트 초기화
      console.log(`프록시가 포트 ${port}에서 시작되었습니다.`);
    } catch (error) {
      console.error('프록시 시작 실패:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleStop = async () => {
    setIsLoading(true);
    try {
      await stopProxyV2();
      setIsRunning(false);
      console.log('프록시가 중지되었습니다.');
    } catch (error) {
      console.error('프록시 중지 실패:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const clearEvents = () => {
    setEvents([]);
  };

  return (
    <div className="p-6 bg-white dark:bg-gray-800 rounded-lg shadow-md">
      <h2 className="text-2xl font-bold mb-4 text-gray-900 dark:text-white">Proxy V2 (Hudsucker) 제어</h2>

      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">포트 번호</label>
          <input
            type="number"
            value={port}
            onChange={(e) => setPort(Number(e.target.value))}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
            min="1024"
            max="65535"
            disabled={isRunning}
          />
        </div>

        <div className="flex space-x-4">
          <button
            onClick={handleStart}
            disabled={isRunning || isLoading}
            className="px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isLoading ? '시작 중...' : '프록시 시작'}
          </button>

          <button
            onClick={handleStop}
            disabled={!isRunning || isLoading}
            className="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isLoading ? '중지 중...' : '프록시 중지'}
          </button>
        </div>

        <div className="mt-4">
          <div className="flex items-center space-x-2">
            <div className={`w-3 h-3 rounded-full ${isRunning ? 'bg-green-500' : 'bg-red-500'}`} />
            <span className="text-sm text-gray-600 dark:text-gray-400">상태: {isRunning ? '실행 중' : '중지됨'}</span>
          </div>
        </div>

        {/* 이벤트 로그 */}
        <div className="mt-6">
          <div className="flex justify-between items-center mb-3">
            <h3 className="text-lg font-medium text-gray-900 dark:text-white">실시간 로그 ({events.length})</h3>
            <button
              onClick={clearEvents}
              className="px-3 py-1 text-sm bg-gray-500 text-white rounded hover:bg-gray-600"
            >
              지우기
            </button>
          </div>

          <div className="max-h-64 overflow-y-auto bg-gray-100 dark:bg-gray-700 rounded-md p-3 space-y-2">
            {events.length === 0 ? (
              <p className="text-gray-500 dark:text-gray-400 text-sm text-center py-4">
                프록시를 시작하면 요청/응답 로그가 여기에 표시됩니다
              </p>
            ) : (
              events.map((event, index) => (
                <div key={index} className="text-xs">
                  <div className="flex items-center space-x-2">
                    <span className="text-gray-500 dark:text-gray-400">{event.timestamp}</span>
                    <span
                      className={`px-2 py-1 rounded text-xs font-medium ${
                        event.type === 'request'
                          ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
                          : event.type === 'response'
                            ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                            : 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200'
                      }`}
                    >
                      {event.type.toUpperCase()}
                    </span>
                  </div>
                  <div className="mt-1 text-gray-700 dark:text-gray-300 font-mono break-all">{event.data}</div>
                </div>
              ))
            )}
          </div>
        </div>

        <div className="mt-4 p-4 bg-gray-100 dark:bg-gray-700 rounded-md">
          <h3 className="font-medium text-gray-900 dark:text-white mb-2">사용법:</h3>
          <ul className="text-sm text-gray-600 dark:text-gray-400 space-y-1">
            <li>• 프록시를 시작한 후, 시스템 프록시 설정을 127.0.0.1:{port}로 변경하세요</li>
            <li>• 모든 HTTP/HTTPS 트래픽이 이 프록시를 통해 라우팅됩니다</li>
            <li>• 요청과 응답이 실시간으로 로깅됩니다</li>
            <li>• WebSocket 연결도 지원됩니다</li>
          </ul>
        </div>
      </div>
    </div>
  );
};
