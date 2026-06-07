import { useState } from 'react';
import { FeedbackSeverity } from '../components/FeedbackSnackbar';

export function useFeedback() {
  const [message, setMessage] = useState('');
  const [severity, setSeverity] = useState<FeedbackSeverity>('success');

  const showFeedback = (nextMessage: string, nextSeverity: FeedbackSeverity = 'success') => {
    setSeverity(nextSeverity);
    setMessage(nextMessage);
  };

  const closeFeedback = () => setMessage('');

  return { feedbackMessage: message, feedbackSeverity: severity, showFeedback, closeFeedback };
}
